# vsdd-factory 1.0 design: agents, skills and models

> Amended by `rewrite/BLUEPRINT.md` on 2026-11-23, and again on 2026-11-24 after the second review pass
> (blueprint section 11). Where this document and the blueprint differ, the blueprint wins.

## Amendments from the blueprint

The text below has been edited in place where a roster, a table or a rule changed. This list is the complete set
of changes; where a passage further down still reads differently, this list and section 4 of the blueprint apply.
Items 1 to 15 are the first pass. Items 16 to 30 are the second pass, and they replace parts of items 5, 7, 8, 9
and 12.

1. **Roster: 20 agents.** Two are added. `clerk` (tier `clerk`: Sonnet at low effort; tools `Bash,
   StructuredOutput`; a body of about 15 lines) runs one `vsdd` command and relays its JSON. `scenario-reviewer`
   (reviewer tier; tools READ, RESULT) reviews hidden scenarios against the contracts; it reads `holdout` and
   `spec-product`, never reviews code, and its findings go only to the product owner (SEAM-02, SEAM-12, COV-07,
   COV-21).
2. **Tiers have one source.** `tests/lint/roster.toml` holds four tiers. Agent files must match it, and the
   workflow prelude's tier and role tables are generated from it. Steps set both model and effort from the
   generated table. The lint allows `effort: low` for the clerk tier only (SEAM-15).
3. **Skills: 13, one model-visible.** The `spike` skill is dropped: `/vsdd-factory:spike` is the pipeline's
   workflow. Two person-only skills are added: `decide` (a hook records the decision; the skill relays the
   hook's context and never runs `vsdd decide`) and `drive` (the low-touch driver). `model_visible_skills = 1`
   (SEAM-06, SEAM-23, SEAM-24, COV-01, COV-05, COV-20, OPS-03).
4. **Workflow names.** The twelve of `pipeline.md` 2.3: `ingest`, `discover`, `spec`, `feature`, `plan`, `wave`,
   `story`, `verify`, `release`, `maintain`, `multi`, `spike`. In section 3.2 read `stories` as `plan`;
   `holdout`, `refine`, `harden` and `converge` as `verify`; `fix` as the fast routes of `feature`; `multi-repo`
   as `multi` (SEAM-38, COV-14).
5. **Methods.** Added: `arbitrate` (architect), `score-ideas` (product-owner, architect, adversary),
   `review-scenarios` (scenario-reviewer), `risk-sweep` (researcher), `intake` (product-owner, architect).
   Removed: `wave-plan` (waves are computed by `vsdd plan schedule`), and the `devops` methods `pull-request`,
   `worktrees` and `gates` (the tool merges, opens pull requests and makes worktrees). `devops` no longer
   merges. Every workflow step names its method, and story kinds select the agent and method of the tests and
   implement stages (blueprint 4.6) (SEAM-04, SEAM-09, SEAM-18, COV-06).
6. **Reviewer contract.** Reviewer files carry the severity table of `pipeline.md` 2.7 word for word (lint), the
   finding key rule, and the field names of the pipeline schemas, including `owner`. A reviewer receives the
   method, the tree root, the diff patch, evidence paths, the review-rules path and the items to account for.
   From a second pass it also receives structured claims (key, location, resolution, commit); that is the one
   exception to "nothing from an earlier pass". Pipeline reviewers return findings only and write no report;
   only an on-demand review writes one (SEAM-13, SEAM-14, SEAM-31, OPS-11, OPS-28).
7. **Result fields.** Section 2.6 role fields use the schema's names: `findings`, `coverage`, `claim_checks`,
   `reviewed_sha`; holdout `scores[].satisfaction` and `symptoms`; the spike runner returns the spike-finding
   schema of `pipeline.md` 2.10.5 (SEAM-14, SEAM-20).
8. **Path classes.** `state.md` 2.4 is the only class table. Names used here change: `holdout-reports` becomes
   `holdout-evaluations` (written by code, so the evaluator writes nothing); `evidence` is
   `.factory/local/evidence/**`; `runs` is added and no agent may read it; `config`, `decisions`, `requests`,
   `research-inputs`, `ideas`, `spikes` and `local` are code-only. The spike runner does not write `spikes`
   with a tool: it runs `vsdd spike open` and `vsdd spike record` (SEAM-20, SEAM-35).
9. **Spikes.** Every agent may ask, including `code-reviewer` and `holdout-evaluator`: the request carries the
   asker's agent type and the spike runner inherits the asker's read walls. Every agent file carries the full
   shared block. Those two agents still have no Agent tool, so outside a workflow they return blocked with the
   question (COV-11).
10. **`omitClaudeMd: true` on every agent.** Conventions reach agents through `conventions.md` and commands
    through `[commands]`. `setup` offers to move build conventions out of a project's `CLAUDE.md` (OPS-19).
11. **Browser and web tools.** Navigation and upload tools are inspected by the hook runner, which treats a
    `file:` address as a read. Agents without a shell, and the clerk, cannot read outside the project, the
    plugin and the temporary directory (SEAM-11, OPS-16, OPS-17).
12. **`setup`** runs `vsdd init`, shows the detected configuration for the person to correct (it cannot write
    `factory.toml`), runs `vsdd doctor`, and offers the terminal shim, the permission allow-list and the
    attribution setting (SEAM-08, SEAM-25, COV-04, OPS-31).
13. **Tools and assets.** `export-pdf` runs `vsdd export pdf` and `author diagram` runs `vsdd render diagram`.
    The design-system starter and the demo and scaffold templates live in `<plugin>/assets/` and are cited by
    methods (SEAM-32, COV-12).
14. **Probes.** V1 to V7, plus two more, run before agent files are written: an agent that lists the Agent tool
    still has none as a workflow step; an agent with `Bash, StructuredOutput` returns a schema result (COV-21).
15. **Open questions.** Section 6 is superseded by section 2 of the blueprint (decisions D5, D6, D13), whose
    recommendations the user adopted on 2026-11-23.

Second pass (2026-11-24):

16. **Planners have a `vsdd`-only shell.** `product-owner`, `architect`, `ux-designer` and `story-writer` get
    Bash, limited by the hook (`tool_only`) to single `vsdd` calls in the read and measure groups, plus
    `feature route` for the architect. They finish a step with the check for their own work. The researcher
    keeps no shell (section 2.5; P2LEAN-03).
17. **Write scopes follow seven product classes.** `tests`, `harness`, `ci`, `repo-config`, `manifests`,
    `docs`, `source`, one class per path. Implementer: `source`, `manifests`. Test-writer: `tests`,
    `manifests`, `source` before red. Verifier: `harness`, `tests`. Devops: `ci`, `repo-config`, `manifests`,
    and new `source` and `tests` files in a `repo-setup` chore story (section 2.9; P2SEAM-01, P2COLD-02,
    P2OPS-03).
18. **Section 2.6 is rewritten** in the schema's field names. No pipeline reviewer is given a report path;
    the implementer is given findings inline; `devops` returns nothing about worktrees, pull requests or
    merges; the story-writer writes no wave plan; the holdout-evaluator writes nothing (P2SEAM-09, P2OPS-09,
    P2COLD-32).
19. **Methods.** Added: `brief`, `recover-contracts` (product-owner); `verification-assessment`,
    `conventions` (architect); `equivalence-tests`, `test-fixes` (test-writer); `pattern-sweep`
    (code-reviewer); `docs-drift` (technical-writer). `stubs` is merged into `failing-tests`. The reference to
    a `release` method is removed. `pipeline.md` 2.9.13 names the method of every step (section 2.8;
    P2SEAM-08, P2COV-03, P2COLD-26).
20. **One text for severity and for the shared block.** `workflow-lib/severity.md` and
    `workflow-lib/agent-shared.md`, stamped into agent files by the build; the example file shows the stamped
    table. `demo-recorder` can own a finding (section 2.7; P2SEAM-10, P2COLD-22).
21. **Walls.** No caller reads `runs`, the main session included. The spike runner, and agents with a
    `vsdd`-only shell, are denied reads outside the project. The stale paragraph about `zones.toml` and
    `roles.toml` is replaced by a pointer to `hooks.md` 2.7 (section 2.9; P2SEAM-11, P2OPS-13, P2COLD-07).
22. **Spikes.** Every agent except the spike runner and the clerk has the Agent tool, limited to the spike
    runner, so `code-reviewer` and `holdout-evaluator` ask for themselves and nobody runs a spike on their
    behalf. The asker's type comes from hook-written records (section 2.11; P2COV-01, P2REQ-10).
23. **The lessons register is removed.** The retrospective method writes its proposals into its report;
    `/vsdd-factory:author conventions` or `author review-rules` adopts one (sections 2.8, 2.9, 2.12;
    P2LEAN-09, P2COLD-31, P2SEAM-14).
24. **`setup`.** It asks the decision D4 question before `vsdd init` and passes the answer as `--auto-allow`;
    supports `--hub`; dispatches the architect to move conventions out of `CLAUDE.md`; offers to archive old
    process sections; on pre-1.0 state `init` converts nothing (section 2.12; P2COV-02, P2OPS-02, P2OPS-15,
    P2COLD-14, P2REQ-05, P2REQ-17).
25. **`brief`.** It saves the draft after each section and has the `product-owner` write the final text
    before `vsdd checkpoint brief` (section 2.12; P2REQ-13, P2REQ-16).
26. **`drive` and `decide`.** The driver pauses at a decision and resumes when it is settled. Choices use
    named values (section 2.12; P2OPS-06, P2SEAM-13).
27. **`defer`, `review`, `research`.** `vsdd defer add` and `vsdd retire`; `vsdd review prepare`; superseded
    reports retired (section 2.12; P2SEAM-14, P2LEAN-07).
28. **Model policy.** `[overrides]` in the roster for the two ingest inventory stages; phases name no model
    (sections 2.3, 2.12; P2COLD-10, P2REQ-13).
29. **Mapping.** Rows 5, 53 and 91 of section 3.2 are corrected (P2COV-03, P2COV-06, P2COLD-26).
30. **Jobs.** No agent carries a job id: a long command is run again while it returns `pending` (sections
    2.4, 2.7; P2LEAN-05, P2COV-05).

Status: design for approval. Written 2026-11-23 by the agents-and-skills designer, in parallel with four other
designers (pipeline, state, hooks, packaging). Names that belong to another design are marked "assumed" where they
first appear and are listed again in section 5 so that a mismatch can be found. Drafts of the packaging and hooks
designs appeared while this document was being finished; where it uses one of their names (`vsdd`,
`factory-hooks`, `roster.toml`, `roles.toml`) it says so. The pipeline and state designs were not visible.

## 1. Summary

In 1.0 this subsystem is 20 agents, 13 skills and about 75 method files, down from 35 agents and 127 skills.
An agent type exists only where the model tier, the tool list, the information wall or the isolation differs.
Coordinators, bookkeepers and relays are gone, because workflows orchestrate and code keeps the books.
Seven agents run on the planner tier, six on the builder tier, six on the reviewer tier and one, the clerk,
on a tier of its own.
Every agent file is short (at most 60 lines), lists its tools explicitly, and points to method files that are read
only when a task needs them.
Walls and write scopes are tables keyed on the calling agent's type, for the hook runner to enforce.
Only one skill is visible to the model (`status`). The other twelve are typed by a person, and no skill
exists merely to start a workflow, because a workflow is already its own slash command; `/vsdd-factory:spike`
is a workflow.
Research and browser tools reach agents through explicit tool lists that name both the plugin's and the user's MCP
servers.
An agent that needs a spike dispatches `vsdd-factory:spike-runner` when it has the Agent tool and returns a
spike request when it does not.

## 2. The design

### 2.1 Terms

| Term | Meaning in this document |
|---|---|
| Agent | A file `<plugin>/agents/<name>.md`. Claude Code runs it as a subagent with its own context, system prompt, model and tool list. Its type name is `vsdd-factory:<name>`, which is also the value hooks receive as `agent_type`. |
| Main session | The conversation a person is typing into. Hooks see it with an empty `agent_type`. |
| Workflow step | One `agent()` call in a workflow script. It runs an agent type and gets a structured result back. A workflow step has no Agent tool (verified by the platform spike). |
| Skill | A directory `<plugin>/skills/<name>/SKILL.md`, invoked as `/vsdd-factory:<name>`. It either instructs the main session or runs in a forked agent (`context: fork`). |
| Model-visible skill | A skill whose description is listed to the model so the model can invoke it. Every agent that has the Skill tool pays for that listing on every start. A skill with `disable-model-invocation: true` is not listed and only a person can run it. |
| Method file | A plain Markdown file `<plugin>/methods/<name>.md` that holds the procedure for one kind of task. An agent reads it with the Read tool when its task names it. It costs nothing until it is read. |
| Tier | One of the three model settings in section 2.3. |
| Wall | A rule that an agent type may not read a class of paths. |
| Write scope | The classes of paths an agent type may change with Write or Edit. |
| Path class | A named set of paths (for example `holdout`). This document names the classes; the state design maps each to real globs. |

### 2.2 Design rules

These rules decided every entry in the rosters.

1. **An agent type must earn its place.** A separate agent exists only where one of four things differs: the
   model tier, the tool list, the wall or write scope, or the isolation. Two old agents that differ only in what
   they are asked to do become one agent with two method files.
2. **No coordinators, bookkeepers or relays.** A workflow script decides what runs next. Status, indexes and
   counts are written or derived by code. An agent that only passes commands along (the old `github-ops`) is
   replaced by giving the tool to the agent that needs it.
3. **Every agent lists its tools.** No agent inherits the session's tool set. This keeps unrelated tools (the
   user's mail, calendar and other MCP servers) out of every agent, and makes the tool list reviewable.
4. **No agent has the Skill tool.** Procedures are method files, and spikes use the Agent tool or a request.
   This removes the skill listing from every agent start if the platform omits the listing when the tool is absent
   (probe V4 in section 7 checks this; nothing depends on it for correctness).
5. **Done means on disk.** An agent writes its artifact (and commits code) before it returns. Its result carries
   paths and facts, not content. No agent writes status, index or log files.
6. **Verdicts are computed, not chosen.** A reviewer reports findings with severities, and the holdout evaluator
   reports one score per scenario. Whether a pass is clean or a gate passes is arithmetic in the workflow script.
7. **A skill must need a person or the main session.** Anything that is a sequence of agent steps is a workflow.
   A skill is kept only for conversation with a person, for a one-off specialist job outside the pipeline, or
   for driving a local tool.
8. **Short files, checked by lint.** Limits in this document (line counts, description lengths, the number of
   model-visible skills) are test failures, not advice. Section 4 lists the checks handed to the packaging design.

### 2.3 Model tiers

| Tier | Frontmatter | Used for |
|---|---|---|
| planner | `model: fable`, `effort: xhigh` | Planning, thinking and creative roles: specification, architecture, UX, story decomposition, codebase analysis, research, spikes. |
| builder | `model: sonnet`, `effort: xhigh` | Everything else: tests, implementation, verification harnesses, demos, documentation, operations. |
| reviewer | `model: opus`, `effort: xhigh` | The adversary, the security reviewer and every other independent reviewer. |
| clerk | `model: sonnet`, `effort: low` | The `clerk` only: steps that run one `vsdd` command and relay its output. They make no judgment. |

- **`tests/lint/roster.toml` is the single source of truth for model and effort.** Each agent file must carry
  its tier's values (lint), so a directly dispatched agent gets them. The build generates the tier and role
  tables in the workflow prelude from the same file, so a workflow cannot disagree with an agent file.
- **Workflow steps set both `model` and `effort`, from the generated table.** Whether a step inherits the
  agent file's effort is not verified, and a generated table is also where the end-to-end tests switch to
  cheap models.
- **Aliases, not pinned model IDs.** `fable`, `sonnet` and `opus` follow the current model of each family (Fable
  5.2, Opus 5.5 and Sonnet 5.5 at the time of writing; all three accept `xhigh`). This is the low-touch choice,
  adopted as decision D5. The weekly conformance job records the model ID behind each alias and runs the
  story tier again when one changes.
- **One marked override.** The roster's `[overrides]` table lists the only steps that run off their agent's
  tier: `ingest/map` and `ingest/analyze`, the broad inventory passes of ingestion, run the
  `codebase-analyzer` on the builder tier (decision D5). The build generates an `OVERRIDE` table into the
  prelude, `step()` consults it, and lint rejects an override the roster does not list. Workflow phase
  metadata names no model.
- **Cost context for tier choices.** At current list prices the planner model costs twice the reviewer model and
  four times the builder model per token. Three roles are judgment calls under the user's policy and are raised as
  open question 1: `codebase-analyzer` and `spike-runner` (planner) and `verifier` (builder).
- **Cross-vendor review is not part of 1.0.** The old documents promised a GPT adversary and a Gemini reviewer
  that never existed on Claude Code. In 1.0 a reviewer differs from a builder by model (Opus against Sonnet) and,
  more importantly, by what it is allowed to see (section 2.9).

### 2.4 The agent roster

| # | Agent | Role | Tier | Replaces |
|---|---|---|---|---|
| 1 | `product-owner` | Writes the product specification chain: domain spec, requirements with behavioral contracts, and the hidden holdout scenarios. | planner | product-owner; business-analyst (domain spec) |
| 2 | `architect` | Designs the architecture and records decisions, verification properties, service-clone and port assessments, data models, and the impact of a change. | planner | architect; data-engineer (schema design) |
| 3 | `ux-designer` | Writes the UX spec, flows, design system and design variants. | planner | ux-designer |
| 4 | `story-writer` | Splits specs into epics and stories with traced acceptance criteria and dependencies. Waves are computed by `vsdd plan schedule`. | planner | story-writer |
| 5 | `codebase-analyzer` | Reads an existing codebase and extracts its behaviour, architecture and conventions. | planner | codebase-analyzer |
| 6 | `researcher` | Gathers outside evidence with citations and analyses signals: domain and technology research, market intelligence, feedback, analytics, opportunity scoring, retrospectives. | planner | research-agent; business-analyst (market analysis); session-reviewer |
| 7 | `spike-runner` | Runs one spike in an isolated worktree and records the finding. | planner | new in 1.0 |
| 8 | `test-writer` | Writes compilable stubs and failing tests from behavioral contracts, plus component, end-to-end and regression tests. | builder | test-writer; stub-architect; e2e-tester (test authoring) |
| 9 | `implementer` | Makes failing tests pass by strict TDD, fixes findings, ports code, and implements migrations, SDKs and service clones. | builder | implementer; data-engineer (migrations) |
| 10 | `verifier` | Builds and runs verification harnesses (proofs, fuzzing, mutation testing, security scans, benchmarks, clone fidelity) and reports measurements. | builder | formal-verifier; performance-engineer; dtu-validator; e2e-tester (Core Web Vitals) |
| 11 | `demo-recorder` | Records demo evidence for each acceptance criterion by driving the built product in a terminal or a browser. | builder | demo-recorder; e2e-tester (journey runs) |
| 12 | `technical-writer` | Writes documentation that describes current behaviour: user docs, the project's CLAUDE.md section, release notes. | builder | technical-writer |
| 13 | `devops` | Builds chore stories: project scaffold, CI, repository and toolchain configuration; runs the observability stack. It does not merge, open pull requests or make worktrees; `vsdd story deliver` and `vsdd survey` do. | builder | devops-engineer; dx-engineer; github-ops |
| 14 | `adversary` | Attacks specs, stories and implementations against the spec with fresh context, and checks brownfield extraction against the code. | reviewer | adversary; spec-reviewer; validate-extraction; consistency-validator (semantic checks) |
| 15 | `code-reviewer` | Reviews a code diff with no access to specs or factory state, as an outside engineer would. | reviewer | code-reviewer; pr-reviewer |
| 16 | `security-reviewer` | Reviews a diff or a system for security defects, triages scanner output, and classifies findings by CWE and OWASP category. | reviewer | security-reviewer |
| 17 | `holdout-evaluator` | Scores the running product against hidden scenarios without seeing source, tests or specs. | reviewer | holdout-evaluator |
| 18 | `ui-reviewer` | Reviews the user interface through a browser: accessibility, visual fidelity, responsive behaviour, usability heuristics, completeness. | reviewer | accessibility-auditor; visual-reviewer; e2e-tester (responsive checks) |
| 19 | `scenario-reviewer` | Reviews the hidden scenarios against the contracts they cite. It never reviews code, and its findings go only to the product owner. It exists because its wall differs from every other agent's: it may read `holdout`. | reviewer | spec-reviewer (its review of scenarios) |
| 20 | `clerk` | Runs one `vsdd` command and returns its JSON output unchanged. While the output says `pending` it runs the same command again. | clerk | state-manager and pr-manager relays, now one command each |

**Old agents that disappear, and why.**

| Old agent | Why it is gone |
|---|---|
| orchestrator (and its nine sequence files) | A workflow script is the orchestrator. The sequences were three prose copies of the pipeline that an LLM dispatched by hand. |
| state-manager | It was a bookkeeper (92 of the old workflow steps). In 1.0 each agent writes its own artifact, gate evidence is written by code, the status is computed from records, and indexes are derived. Nothing is left for it to do. |
| pr-manager | It was a nine-step coordinator that spawned five other agents. The steps become steps of the story workflow, and the `git` and `gh` work is done by the `vsdd story deliver` command. |
| github-ops | It was a relay that ran `gh` for agents without a shell. The tool runs `gh` itself. |
| consistency-validator | Its mechanical checks (identifiers resolve, traces exist, counts agree) become a deterministic lint tool. Its semantic checks become part of the adversary's spec review. |
| spec-steward | Versioning is git history, traceability is a generated report, and governance of spec changes is the feature workflow. |
| stub-architect | The old workflows already gave stub writing to test-writer; the separate agent was never dispatched. |
| spec-reviewer, pr-reviewer, validate-extraction, session-reviewer, business-analyst, data-engineer, dx-engineer, dtu-validator, performance-engineer, formal-verifier, e2e-tester, accessibility-auditor, visual-reviewer | Merged, because after the cross-vendor fiction is removed they share a tier, a tool list and a wall with the agent that absorbs them. Their procedures survive as method files. |

**Why these merges are safe and where the line is.** `devops`, `verifier` and `researcher` each absorb several
old agents. The rule for splitting one again is mechanical: split when two of its tasks need different tools, a
different wall, or a different tier. Until then, a method file is the cheaper way to carry a different procedure,
because it adds nothing to the fixed cost of every agent start.

### 2.5 Tools

Tool lists are written out in each agent file. This table uses named sets so the roster stays readable; the
agent-file lint checks each file against it.

| Set | Tools |
|---|---|
| READ | `Read`, `Grep`, `Glob` |
| EDIT | `Write`, `Edit` |
| REPORT | `Write` only. The hook runner limits it to creating new files in the agent's own report directory. |
| SHELL | `Bash` |
| VSDD | `Bash`, limited by the hook runner to single calls of the `vsdd` tool (`tool_only` in the agent's wall row), in the command groups the row grants. It is a way to finish a step with a check, not a shell: no pipe, no redirect, no other program. |
| SPAWN | `Agent`. The hook runner limits the callee to `vsdd-factory:spike-runner` (section 2.9). A workflow step never has this tool, whatever the file says. |
| RESULT | `StructuredOutput`. Listed defensively so that an explicit tool list cannot hide the tool a workflow step returns its result with (probe V2). |
| WEB | `WebSearch`, `WebFetch` |
| DOCS | Context7: `resolve-library-id`, `query-docs` |
| LOOKUP | Perplexity: `perplexity_ask`, `perplexity_search` |
| DEEP | Perplexity: `perplexity_research`, `perplexity_reason`. Tavily: `tavily_search`, `tavily_extract`, `tavily_crawl`, `tavily_map`, `tavily_research` |
| BROWSER | Playwright: `browser_navigate`, `browser_navigate_back`, `browser_snapshot`, `browser_take_screenshot`, `browser_click`, `browser_type`, `browser_fill_form`, `browser_select_option`, `browser_hover`, `browser_press_key`, `browser_wait_for`, `browser_resize`, `browser_console_messages`, `browser_close` |
| STORYBOOK | Storybook MCP: `list-all-documentation`, `get-documentation`, `get-documentation-for-story`, `get-storybook-story-instructions`, `preview-stories` |

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

Two changes from the first pass. The four planners have VSDD. Without it every check of their work needed a
clerk, and a failed check started a new planner with no context: a fast-route feature spent four of its six
non-story agent starts on clerks. The cost is that planners are no longer free of the Bash tool, which
slightly widens what a prompt-injected document could reach; the hook limits it to read and measure
commands whose inputs come from `factory.toml`, which no tool call can write. The researcher, which reads the
most untrusted text, keeps no shell. And every agent except the spike runner and the clerk has SPAWN, so that
a walled reviewer dispatched in a normal session asks for a spike itself, and the hook records who asked.

### 2.6 What each agent is given, writes and returns

Every result also carries the common fields of the pipeline design's step-result schema: a status (done,
blocked, failed, or a spike is needed), a summary, the commit left behind, the list of artifacts written, the
gate results, and the spike request when one is needed. The pipeline design owns the JSON Schemas
(`pipeline.md` 2.10), and their field names are the contract. The table below is written in those names, so
an agent file's Result section can be built from its row. Package A6 builds the reviewer files from this
table.

Three rules hold for every row. An agent is given findings and claims inline, in its prompt, never as the
path of a review report, because pipeline reviews write no report. A pipeline reviewer is given no report
path and writes no file; a report path appears only in an on-demand review started by the `review` skill,
and the "on demand" entries below mean exactly that. And an agent that finishes with a gate puts the gate's
JSON in `gates`; it does not report whether it passed.

| Agent | Is given | Writes | Returns in `data` |
|---|---|---|---|
| `product-owner` | Method name; paths of the brief, research and any brownfield analysis; for a fix, the findings (inline) and their keys; for a revision, the person's guidance; for the scenario review, that loop's findings and claims, which only it receives. | Domain spec, requirements, behavioral contracts and supplements; hidden scenarios; the final text of the brief. | `ids_added`, `ids_changed`, `ids_retired` (contract identifiers); `resolutions` when fixing (`key`, `resolution`, `commit`, `note`); `covers` after an intake (stage names); `requests` (work for another role). |
| `architect` | Method name; product spec paths; for delta analysis, the change request path; for a fix, the findings inline; for arbitration, the open findings and the builder's dispute notes. | Architecture sections, decision records, verification properties, assessments, `dependencies.toml`, `toolchain.toml`, `conventions.md`, `review-rules.md`. | `decisions`, `properties`; `feasibility` (`ok`, `issues`); delta data for `delta-analysis` (`pipeline.md` 2.10.8), with the route gate result in `gates`; `rulings` for `arbitrate` (`key`, `ruling`, `reason`); `requests`; `clones_required`, `port_candidates`. |
| `ux-designer` | Method name; product spec paths; design system path; for a fix, the findings inline. | UX spec, design system. | `screens`, `flows`, `components`; `resolutions` when fixing; `requests`. |
| `story-writer` | Method name; spec paths; existing stories; for a fast route, the delta record path and the readiness stories to write; for a fix, the findings inline. | Epics and stories. It writes `depends_on` and `priority`; it writes no wave plan, because `vsdd plan schedule` computes the waves. | Outline data for `decompose` (`epics` with `stories`); `stories` (identifiers); `oversized` (stories it could not split); `untraced` (contracts no story covers); on a fast route, where it ran `vsdd gate plan --story` once for each story it wrote, those gate results in `gates` and `units`, the admitted story unit of each story, copied from its gate result's `unit` (required by the step's data schema, `pipeline.md` 2.10.8). |
| `codebase-analyzer` | Method name; codebase path; the module or the targets, verbatim from the workflow; paths of earlier output. | Analysis files. | Module-map data for `ingest-broad` (`modules`); `files_covered`, `files_total`; `low_confidence` (areas it could not establish). |
| `researcher` | Method name; the question or topic; paths of earlier research and of collected inputs. | One dated report (a retrospective is one, with its proposals). | `artifacts` holds the report path; `sources_count`; `tools_used` (tool name to call count); `inconclusive` (questions left open); `superseded` (paths of earlier reports this one replaces, which the checkpoint retires); score data for `score-ideas`. |
| `spike-runner` | The question, why it blocks the asker, a suggested method, the starting commit, and its opening command. Not who asked: the tool takes that from a hook-written record. | Experiment files inside its own worktree, and the question or finding file in the temporary directory. The ledger entry is written by `vsdd spike open` and `vsdd spike record`, not by a tool call. | The spike-finding schema of `pipeline.md` 2.10.5, as its whole result: `ledger_id`, `question`, `verdict` (`confirmed`, `refuted`, `answered`, `inconclusive`, `refused`), `answer`, `method`, `evidence`, `confidence`, `cached`, `artifacts`, `follow_up`. |
| `test-writer` | Method name; story path; worktree path; contract paths; for a fix, the findings inline (categories `test` and `coverage`). | Test files and the stubs they need; commits. | The red gate result in `gates` (and, after a fix, the re-freeze and green results); `tests_added`; `criteria_covered`; `criteria_uncovered` (with the reason); `resolutions` when fixing. |
| `implementer` | Method name; story path; worktree path; for a fix, the findings (inline) and their keys; for a repair, the delivery state and detail. | Source files and manifests; commits. | The green gate result in `gates`; `resolutions` when fixing (`key`, `resolution`: `fixed` or `disputed`, `commit`, `note`); `spec_gaps`; `test_defects` (tests it believes are wrong and may not edit); `concerns`. |
| `verifier` | Method name; the story and worktree paths of the harness story; property identifiers; thresholds from project configuration. | Harness files and tests; commits. It measures nothing itself: the checks run as `vsdd gate` jobs. | The green gate result in `gates`; `harnesses` (property or target, path); `not_buildable` (with the reason). |
| `demo-recorder` | Story path; worktree path; the criteria to show; how to start the product (`vsdd env start`). | Demo scripts, transcripts and small screenshots under `demo/`. | The demo gate result in `gates`; `criteria_with_evidence`; `criteria_missing` (with the reason). |
| `technical-writer` | Method name; scope; source paths; for `docs-drift`, the documentation paths to compare with the code. | Documentation files. | The green gate result in `gates` in a `docs` story; `files`; `unverified` (behaviour it could not confirm from the code); sweep data for `docs-drift` (`findings`, `fixable`). |
| `devops` | Method name; the chore story and its worktree path; what the story must set up. | CI, repository configuration and manifests; in a `repo-setup` story, the first source and test files of the scaffold; commits inside the story worktree. It runs no `git push`, opens no pull request, merges nothing and makes no worktree: `vsdd story deliver` and `vsdd survey` do. | The green gate result in `gates`; `files`. |
| `adversary` | Method name (the lens); the tree root and target paths; evidence paths (the latest diff patch, test output) for code; scope; the items to account for; the project's review rules file; from a second pass, structured claims to verify. Never an earlier review and never a builder's summary. On demand only: a report path. | Nothing in the pipeline. On demand only: one review report at the path it was given. | Review data: `reviewed_sha`; `findings` (each with `key`, `title`, `severity`, `category`, `scope`, `owner`, `location`, `spec_ref`, `repro`, `observed`, `expected`); `claim_checks` (`key`, `outcome`, `note`); `coverage` (`item`, `state`, `note`). A defect outside the scope it was given is a finding whose `scope` names the level that should handle it. |
| `code-reviewer` | The diff patch path; the tree root; the review rules file; the items to account for; claims from a second pass. For `pattern-sweep`: the tree root and `conventions.md`, and no diff. On demand only: a report path. | Nothing in the pipeline. On demand only: one review report. | Review data, as `adversary`. Sweep data for `pattern-sweep` (`findings`, `fixable`). |
| `security-reviewer` | Method name; the diff patch or module list; scanner output paths; architecture path; claims from a second pass. On demand only: a report path. | Nothing in the pipeline, except `analysis/<scope>/security.md` once, during an ingest. On demand only: one review report. | Review data, as `adversary`, with a CWE identifier in each finding's `title`. |
| `holdout-evaluator` | The scenario identifiers to run; how to start the product (`vsdd env start`). No source, test or specification path. | Nothing. Its evaluation file is written by code from its result. | Holdout data: `scores` (`scenario`, `satisfaction` from 0 to 1, `must_pass`, `ran`); `symptoms` (`scenario`, `observed`, `expected`, with no scenario text and no location). It returns no findings: the tool builds them from the scores, and the workflow never asks it for a gate. |
| `ui-reviewer` | Method name and the lens; how to start the product (`vsdd env start`), or a URL; UX spec and design system paths; the items to account for; claims from a second pass. On demand only: a report path. | Nothing in the pipeline. On demand only: one review report with screenshots. | Review data, as `adversary`; `measurements` (for example contrast failures, breakpoints checked). |
| `scenario-reviewer` | Method `review-scenarios`; the scenario paths and the contract paths they cite; the items to account for; claims from its own earlier pass. | Nothing. | Review data, as `adversary`. Its findings quote scenarios, so they are walled: they go only to the `product-owner`, in a loop of their own, and are stored in the holdout class. |
| `clerk` | One `vsdd` command. | Nothing. | The command's JSON, unchanged. |

Each finding names an `owner`, the role that should fix it. The workflow routes on that field. This replaces
the old `pr-review-triage` skill and the orchestrator's routing table.

### 2.7 The standard agent file

An agent file has a six-key frontmatter and six sections: the opening paragraph, Rules, Inputs, Method, Result
and the shared block. Lint rejects any other key and any file whose body is longer than 60 lines; the stamped
regions (the shared block, and in reviewer files the severity table) are not counted in the 60.

```markdown
---
name: <kebab-case, no colon>
description: <one sentence, at most 200 characters: what it does and when to use it>
model: <fable | sonnet | opus>
effort: xhigh
tools: <explicit comma-separated list; never omitted>
color: <blue for planner | green for builder | red for reviewer>
---

You are the <role> of a vsdd-factory project. <Two or three sentences: the mission and what a good result is.>

## Rules
<At most eight. Each is specific to this role. Each is either enforced by a hook (say "enforced") or visible in
the result.>

## Inputs
<What the caller must give you. If a required input is missing, return blocked; do not look for a substitute.>

## Method
Your task names a method. Read `${CLAUDE_PLUGIN_ROOT}/methods/<name>.md` before you start.
<One line per method this agent has.>

## Result
<The role fields from section 2.6.> Fill in the result schema you were given. If you were given none, end your
reply with the same facts under the headings STATUS, SUMMARY and ARTIFACTS.

<the shared block, below>
```

Two more keys exist. `omitClaudeMd: true` is set on every agent: a project's CLAUDE.md can run to hundreds
of lines of process rules, it would be loaded into every agent start, and for the holdout evaluator it
describes internals it must not see. Project conventions reach agents through `conventions.md` and build
commands through project configuration. `isolation: worktree` is set on `spike-runner`.

**The shared block** is the same text in every agent file, checked byte for byte by lint. Agent files cannot
import, so the build stamps it: the one source is `workflow-lib/agent-shared.md`, and `tools/build.py` copies
it into every agent file between markers, as it stamps the prelude into workflows. One agent omits the first
bullet of the second section, because a spike cannot start a spike: `spike-runner`. The `clerk` carries a
two-line block of its own. Reviewer files also carry the severity table and the finding key rule
(`<category>:<path>:<symbol or clause>`, the same in every pass). The severity table has one source too,
`workflow-lib/severity.md`, which holds the table of `pipeline.md` section 2.7 and is stamped into every
reviewer file under the heading `## Severity`. The first pass had three wordings of that table (the pipeline
design, the blueprint and the example below), which a byte-for-byte lint could never have passed; now the
example shows the stamped text and nothing paraphrases it.

```markdown
## Ground rules
- The spec is the authority. If it is wrong or silent, say so in your result. Do not work around it.
- Report only what you checked. Name the command you ran or the file you read. If a check did not run, say so.
- Done means on disk. Write your work, and commit code, before you return. Your result points to it.
- Stay in your role. If a fix belongs to another role, describe it in your result and leave it.
- Use the paths you were given, as given. Do not build a path into the state directory yourself.
- If a hook blocks a tool call, read its reason and follow it. Do not reach the same effect another way.

## When you cannot proceed
- A question that only an experiment or outside research can settle: do not guess. If you have the Agent tool,
  dispatch `vsdd-factory:spike-runner` with the question, why it blocks you and a time box, then continue with
  its finding. If you do not have the Agent tool, you are a workflow step: stop and return the spike request
  your result schema provides.
- A missing input, or a problem you could not fix in two attempts: stop and return blocked, with what you tried.
```

**A complete example.** The old `adversary.md` was 384 lines. This is the whole 1.0 file apart from the shared
block.

```markdown
---
name: adversary
description: Attacks specs, stories or an implementation against the spec with fresh context and reports concrete defects. Use for any independent review that needs the spec.
model: opus
effort: xhigh
tools: Read, Grep, Glob, Write, Agent, StructuredOutput
color: red
---

You are the adversary of a vsdd-factory project. You look for real defects: behaviour the spec does not cover,
contradictions, and places where an artifact does not do what the spec says. You have not seen any earlier review
and you do not want to: your value is that you start cold. The one thing you may be given from an earlier pass is
a list of claims (a finding key, its location, "fixed" or "disputed", a commit): check each against the code.

## Rules
- Report defects only. No praise, no summary of what is good, no style preferences.
- Every finding names a file and line you read, says what breaks, and names the role that should fix it.
- A critical or high finding against code needs proof from the code: quote it, or say where you searched and
  found nothing.
- Review only the scope you were given. A defect outside it is still a finding; set its `scope` to the level
  that should handle it.
- Give a `coverage` entry for every item you were asked to account for: `checked`, `not_checkable` with the
  reason, or `skipped`. A review that read nothing is a failure, not a clean pass.
- You cannot read earlier reviews or holdout scenarios (enforced). Write a report file only if your task gives
  you a report path, which happens only in an on-demand review.
- Assign severity by the table below. You do not decide whether the pass is clean; the workflow counts.
- Give each finding a stable key, `<category>:<path>:<symbol or clause>`, and name its `owner`.

## Severity
<!-- >>> stamped:severity (from workflow-lib/severity.md; do not edit) -->
| Severity | Definition |
|---|---|
| `critical` | The change breaks a stated contract in normal use, loses or corrupts data, opens an exploitable security hole, or breaks the build or the suite. |
| `high` | A stated contract clause, acceptance criterion or listed edge case is unmet or untested, or a test cannot fail. |
| `medium` | A real defect with a concrete consequence that no contract covers. |
| `low` | Style, naming, wording, preference. |
<!-- <<< stamped:severity -->

## Inputs
A method name, the tree root and target paths, the evidence paths when reviewing code, the items to account
for, the project's review rules file if there is one, and claims to verify if this is not the first pass.

## Method
Your task names one of `review-specs`, `review-stories`, `review-story-diff`, `review-wave`, `review-system`,
`review-extraction`. Read `${CLAUDE_PLUGIN_ROOT}/methods/<name>.md` before you start.

## Result
Return `reviewed_sha`, `findings`, `claim_checks` and `coverage` in the result schema you were given. If you
were given a report path, also write the report there.
```

**What the old files carried that the new ones do not.** The Identity block and emoji, the agent ID line, the
pointers to `FACTORY.md` and `VSDD.md` (which did not resolve), the Tool Access section in a dead platform's
vocabulary (the frontmatter is now the truth), the three escalation levels, the Remember line, the footer, the
context-discipline lists (the caller passes exact paths and the walls are enforced), tier labels T1 to T3, phase
numbers, model and vendor claims, and paths belonging to other projects. Lessons and procedures moved to method
files. Section 3.4 maps each old section.

### 2.8 Method files

A method file is the procedure for one kind of task: when to use it, the inputs, the steps, the output and its
template, and the checks to make before returning. It is at most 150 lines. A workflow step names it in the
step prompt, and a forked skill names it in the skill body, so each procedure has one source. Method files replace
the bodies of the old skills, their 148 supporting step files, the nine orchestrator sequence files, the ten
rule files and most of each old agent file.

Why files and not skills: a skill the model can load must be model-visible, and every model-visible skill is
paid for by every agent start. A method file is paid for only by the agent that reads it.

| Agent | Method files (`<plugin>/methods/<name>.md`) |
|---|---|
| `product-owner` | `domain-spec` (also the project context after an ingest), `requirements` (contracts, error taxonomy, interfaces, non-functional requirements, module criticality, revision after feasibility review), `holdout-scenarios`, `spec-delta` (feature-mode spec evolution, traceability, contract retirement), `recover-contracts` (contracts recovered from the analysis of existing code, marked `origin: recovered`, with scenario seeds), `brief` (the final text of the brief, from the draft the `brief` skill saved), `intake` (converting existing documents; returns `covers`), `score-ideas` |
| `architect` | `architecture` (including `dependencies.toml` and `toolchain.toml`, and after an ingest the recovered architecture), `decision-record`, `verification-properties`, `feasibility`, `clone-assessment` (the old DTU assessment), `port-assessment` (the old gene-transfusion assessment), `verification-assessment` (the verification gap analysis of existing code), `conventions` (`conventions.md` and `review-rules.md`: writing them after an ingest, moving build conventions out of a project's `CLAUDE.md`, and adopting a retrospective's proposal), `delta-analysis` (ends with `vsdd feature route`), `data-model`, `multi-repo-synthesis`, `diagram`, `arbitrate`, `intake`, `score-ideas` |
| `ux-designer` | `ux-spec`, `design-system`, `design-variants` |
| `story-writer` | `decompose`, `story` (create, refine, split; the completeness checklist; on a fast route it ends with `vsdd gate plan --story`, and it writes the readiness chore stories the route lists and one story per repository touched). There is no `wave-plan`: the story-writer writes `depends_on` and `priority`, and `vsdd plan schedule` computes the waves. |
| `codebase-analyzer` | `ingest-broad` (the module map), `ingest-deepen` (per-module analysis, and corrections after validation), `ingest-synthesis`, `port-analysis` (the old semport analysis), `disposition` |
| `researcher` | `research`, `market-intel`, `discovery` (competitive monitoring, feedback and analytics signals read from `research/inputs/`, synthesis, idea briefs, post-feature validation), `risk-sweep`, `retrospective` (one research report of kind `retrospective` with improvement proposals, each naming the line it would add to `conventions.md` or `review-rules.md`; it returns `superseded` and writes no lessons file) |
| `spike-runner` | `spike` |
| `test-writer` | `failing-tests` (a failing test for every criterion, with the stubs the tests need to build; the first pass's separate `stubs` method is merged in, because one step takes one method), `regression-test`, `equivalence-tests` (behavioural-equivalence tests against a reference implementation, for `port` stories), `test-fixes` (fixing findings of category `test` or `coverage`: strengthening a test that cannot fail, adding the test for an uncovered criterion; it ends with `vsdd gate red --refreeze` and then runs the green gate, whose result it returns whether or not it passes: a strengthened test that fails on the current code is re-frozen, and the implementer makes it pass), `ui-tests`, `e2e-tests` |
| `implementer` | `tdd`, `fix-findings` (includes root-cause debugging), `port-translation`, `ui-implementation`, `data-migration`, `sdk-generation`, `service-clone` |
| `verifier` | `proofs`, `fuzzing`, `mutation`, `security-scan`, `performance`, `clone-fidelity` |
| `demo-recorder` | `demo` |
| `technical-writer` | `docs`, `docs-drift` (the maintenance sweep: compares the documentation with the code and returns findings and which are mechanically fixable), `claude-md`, `release-notes` |
| `devops` | `repo-setup`, `ci`, `toolchain`, `observability`. Worktrees, gates, pull requests, merges and releases are `vsdd` commands, and issue trackers are read by `vsdd discover collect`, so those methods are gone. |
| `adversary` | `review-specs`, `review-stories`, `review-story-diff`, `review-wave`, `review-system`, `review-extraction`, `score-ideas` |
| `scenario-reviewer` | `review-scenarios` |
| `clerk` | none: its task is the command |
| `code-reviewer` | `review-diff`, `pattern-sweep` (the maintenance sweep: reads the code against `conventions.md` for inconsistent patterns; it needs no diff) |
| `security-reviewer` | `security-review` (a diff, a system, or the triage of scanner and audit output) |
| `holdout-evaluator` | `holdout-eval` (also the re-run half of the scenario-freshness sweep) |
| `ui-reviewer` | `ui-review` (lenses: accessibility, visual, responsive, heuristics, completeness, design drift) |

The scenario-freshness sweep of `maintain` has two halves because of the wall. The evaluator re-runs the
scenarios with `holdout-eval`; it may not read contracts, so it cannot judge whether a scenario is stale or
whether the contracts are still covered. The `scenario-reviewer`, which may read both, judges that with
`review-scenarios`.

One step, one method. `step()` takes exactly one method name, and universal test 6 checks that the method
file exists and that the step's agent file lists it. `pipeline.md` section 2.9.13 gives the method of every
step of every workflow, and it is the table the workflow scripts are written from. There is no `release`
method: the release is one command of the tool, and its notes use `release-notes`.

The build may merge two method files that are always read together. It may not let one grow past the limit.

### 2.9 Information walls

The hook runner enforces these tables by the `agent_type` of the calling agent. The tables use path classes; the
state design maps each class to globs (need N-STATE-1).

**Path classes.**

The state design's table (`state.md` 2.4) is the authority for class names and globs, and is the only
content of `classes.toml`. The rows below are a reading aid in its names.

| Class | Contents | Location (from `state.md` 2.4) |
|---|---|---|
| `holdout` | Hidden acceptance scenarios, and everything under them. | `.factory/holdout/**` |
| `holdout-evaluations` | Holdout evaluations, written by code from the evaluator's result. Inside `holdout`, so every wall on `holdout` covers them. | `.factory/holdout/evaluations/**` |
| `reviews` | On-demand review reports. | `.factory/reviews/**` |
| `runs` | Stored step results. No caller may read them, the main session included; findings are read through `vsdd review show`. | `.factory/local/runs/**` |
| `spec-product` | Product brief, domain spec, requirements, behavioral contracts. | `.factory/specs/product/**` |
| `brief` | The product brief only (a subset of `spec-product`). | `.factory/specs/product/brief.md` |
| `spec-arch` | Architecture, decision records, verification properties. | `.factory/specs/architecture/**` |
| `guidance` | `conventions.md` and `review-rules.md` only (a subset of `spec-arch`). | `.factory/specs/architecture/conventions.md`, `review-rules.md` |
| `spec-ux` | UX spec and design system. | `.factory/specs/ux/**` |
| `stories` | Epics and stories. The wave schedule is computed and stored in a closure record. | `.factory/stories/**` |
| `research` | Research and discovery reports. | `.factory/research/**` |
| `analysis` | Brownfield and port analysis. | `.factory/analysis/**` |
| `evidence` | Gate evidence written by the plugin CLI: command output, diff patches, scanner output, head SHAs. | `.factory/local/evidence/**` |
| `demo` | Demo scripts, transcripts and evidence reports. | `.factory/demo/**` |
| `deferred` | The deferred-work register. There is no `lessons` class: a retrospective is a research report. | `.factory/deferred/**` |
| `config`, `control`, `decisions`, `requests`, `research-inputs`, `ideas`, `spikes`, `local` | Project configuration; notes and cycle records; decisions; change requests; collected inputs; the idea registry; the spike ledger; the local store. No agent writes any of them with a tool. | See `state.md` 2.4 |
| `tests`, `harness`, `ci`, `repo-config`, `manifests`, `docs`, `source` | The project's own tests; verification harnesses (proofs, fuzz targets, benchmarks); CI configuration; toolchain and repository configuration files; build manifests and lock files; documentation; code. A product path belongs to exactly one of them: the first that matches, in this order. | Globs from `[paths]` in project configuration. |

**Read walls.** "May not read" covers the `Read`, `Grep` and `Glob` tools and any `Bash` command that names a path
in the class (need N-HOOK-1), a `vsdd` read command that prints the class, and a browser tool opening a
`file:` address in it. Wherever a row says `holdout`, it covers `holdout-evaluations` too, which is where
the full form of a finding that quotes a scenario is stored. In addition to its row, every caller is denied
`runs`, the main session included; and agents with no shell or a `vsdd`-only shell, the clerk and the spike
runner may not read outside the project, the plugin and the temporary directory.

| Calling agent type | May not read | Why |
|---|---|---|
| `product-owner` | nothing | It writes the holdout scenarios. |
| `architect`, `ux-designer`, `story-writer`, `codebase-analyzer`, `researcher` | `holdout` | Their output is read by builders, so it must not carry scenario content. |
| `test-writer`, `implementer`, `demo-recorder`, `technical-writer`, `devops` | `holdout` | Builders must not build to the hidden scenarios. |
| `verifier` | `holdout`, `reviews` | It verifies independently, not led by what a reviewer already found. |
| `spike-runner` | `holdout`, `reviews`, everything outside the project, and every class the asking agent may not read | Its finding goes back to whoever asked. The ledger entry records the asker's type, which the tool takes from a hook-written record and never from the runner, and the hook applies the union. |
| `scenario-reviewer` | `source`, `tests`, `ci`, `reviews`, `stories`, `analysis`, `spec-arch`, `spec-ux` | It reads `holdout` and `spec-product` to check scenarios against contracts, and nothing it could carry to a builder. |
| `clerk` | `holdout`, `reviews` | It runs commands and reads nothing. |
| `adversary` | `holdout`, `reviews` | Fresh context: it never sees an earlier pass, its own or anyone's. |
| `security-reviewer`, `ui-reviewer` | `holdout`, `reviews` | Independent of the other reviewers. |
| `code-reviewer` | every class in the state directory except `evidence` and `guidance` | It sees the diff, the code, and the project's conventions and review rules, which its steps are given, and nothing that explains the intent. In `walls.toml` the two exceptions are the row's `read_allow`. |
| `holdout-evaluator` | `source`, `tests`, `harness`, `manifests`, `repo-config`, `ci`, `evidence`, `reviews`, `stories`, `research`, `analysis`, `spikes`, `spec-arch`, `spec-ux`, and `spec-product` except `brief` | It judges the product from outside. It may read `holdout`, `brief` and `docs`. |
| main session (empty type) | `holdout` | It composes prompts for other agents. Decision D6, adopted. `[hooks] main_session_wall = false`, which only a person can set, relaxes this row and no other. When the main session dispatches an agent that may read `holdout`, the hook checks that agent's final message for scenario text. |
| any other type (a workflow step with no agent type, a built-in agent, another plugin's agent) | `holdout`, `reviews` | An unknown caller gets the strictest row. |

**Write scopes.** "May write" covers `Write`, `Edit` and `NotebookEdit`. Anything not listed is blocked.

| Calling agent type | May write |
|---|---|
| `product-owner` | `spec-product`; `holdout` |
| `architect` | `spec-arch` |
| `ux-designer` | `spec-ux` |
| `story-writer` | `stories` |
| `codebase-analyzer` | `analysis` |
| `researcher` | `research` |
| `spike-runner` | any path inside its own spike worktree, and the temporary directory. Its ledger entry is written by `vsdd spike open` and `vsdd spike record`, not with a tool |
| `test-writer` | `tests`, including the tests frozen at the red gate; `manifests` (test dependencies); `source` only until the story's failing-test gate is recorded (stubs) |
| `implementer` | `source` and `manifests`; never `tests`. A dependency it adds to a manifest is caught at delivery, where it needs a `merge` decision |
| `verifier` | `harness` and `tests`, but not the tests frozen at a story's red gate |
| `demo-recorder` | `demo` |
| `technical-writer` | `docs` |
| `devops` | `ci`, `repo-config` and `manifests`, inside a story worktree. In a chore story whose method is `repo-setup` (the scaffold of an empty repository) it may also create new files in `source` and `tests` |
| `adversary`, `code-reviewer`, `security-reviewer`, `ui-reviewer` | `reviews`, creating new files only, in an on-demand review. `security-reviewer` may also create `analysis/<scope>/security.md` during an ingest |
| `holdout-evaluator`, `scenario-reviewer`, `clerk` | nothing |
| main session | Inside the state directory: `brief` (the draft, in the `brief` skill) and `deferred` (in the `defer` skill), and nothing else; in particular not `spec-arch`, which is why conventions are moved by dispatching the architect. Outside it: project files in the main checkout, which it may edit but may not commit on the integration or release branch. Decision D6, adopted. |
| any other type | nothing in the state directory |

The builders' grants are written against one class per path. A file that both a source glob and a tests glob
match, such as `src/parser/tests.rs`, is in `tests`, so the test-writer may fix it after red and the
implementer may not touch it. A lint rule (`L-KIND`, in the packaging design) checks the table against the
story kinds: for every kind, the agent of its implement stage can write every class its method's output
needs. That rule would have caught the first pass's gap, in which the scaffold story belonged to an agent
that could write only `.github/**`.

`evidence`, `control`, `decisions`, `holdout-evaluations`, `requests`, `research-inputs`, `ideas`, `spikes`
and `runs` are written only by the plugin CLI and the hook runner. No agent writes them with `Write` or
`Edit`. `config` (`factory.toml`) is written by no tool call at all: `vsdd init` creates it and a person
edits it. Builders write project code only inside a story worktree.

**In the hook design's format.** The two tables above are the content of `walls.toml`, one row per agent
type, in the format of `hooks.md` section 2.7, which is the only description of that format. This document
no longer carries a translation: an earlier one described files (`zones.toml`, `roles.toml`) and flags from a
draft that the hook design replaced. In `walls.toml` a write grant is a class name, with `:create` for new
files only, `:frozen` and `:before-red` for the test-writer, and `@repo-setup` for the scaffold grant of
`devops`; the product classes come from `[paths]`, so every scope in the table above is enforced by the hook
and none is left to instruction.

Two consequences are worth stating. First, the implementer cannot weaken a test to make it pass: it reports the
test under `test_defects` and the workflow sends it to `test-writer`. Second, an architect who finds a contract
infeasible cannot rewrite it: the contract goes back to `product-owner` through `requests`.

**Dispatch.** Checked on `PreToolUse` for the `Agent` tool, from the caller's type and the requested type.

| Caller | May dispatch |
|---|---|
| main session | any agent |
| any `vsdd-factory:*` agent except the two below | `vsdd-factory:spike-runner` only, and only with worktree isolation. The hook records who dispatched it, and the runner inherits that agent's read walls |
| `vsdd-factory:spike-runner`, `vsdd-factory:clerk` | nothing (they also have no Agent tool) |

**Command scopes.** These are proposals to the hook design, which owns `Bash` rules.

| Command family | Allowed for |
|---|---|
| `git push`, `gh pr create`, worktree add and remove, tagging | The main session when a person asks. Inside the pipeline no agent runs them: `vsdd story deliver`, `vsdd survey` and `vsdd release` do. |
| `gh pr merge`, and any merge into or push to the integration or release branch | Nobody. `vsdd story deliver` and `vsdd release` are the only routes. |
| `vsdd` subcommands | Per role, from the `[callers]` table in the hook design's `walls.toml`: the clerk runs pipeline commands; a builder runs the gate for its own story; a planner runs the check for its own work (and the architect `feature route`); the main session runs the read, author and operator groups, which do not include `survey`, `checkpoint` (other than `checkpoint brief`), `story deliver` or `release`; nobody but a person runs `decide`, `config set`, `cycle close` or `lock release --force` |
| `git commit` inside a story worktree | `test-writer`, `implementer`, `verifier`, `demo-recorder`, `technical-writer`, `devops` |
| any `git` command, and any command naming a `source` or `tests` path | not `holdout-evaluator` |
| any push | not `spike-runner`: nothing leaves a spike except through the pipeline |

**Three rules for prompts**, which hooks cannot check and workflow authors must follow (need N-PIPE-3):
a reviewer's prompt contains the method, target paths, evidence paths, scope and the items to account for
(and a report path in an on-demand review), and nothing else; it never contains a builder's summary or an
earlier finding, with one stated exception: from the second pass of a convergence loop it carries claims
as structured data (the finding key, the path and line range of its location but never the quote, `fixed`
or `disputed`, and the commit) with none of the fixer's free text, and a wave reviewer receives the findings
that story reviews deferred as scope to check, not as conclusions; no prompt to an agent that may not read
`holdout` contains text from a walled finding (one from the scenario review, or located under
`.factory/holdout/`), which a universal harness test checks; and when a finding is resolved by a deliberate
decision, the decision is written into the spec or a decision record, so the next fresh reviewer learns it from
the artifact and not from the review trail.

### 2.10 Research and browser tools

A plugin agent cannot declare its own MCP servers. Three facts make the tools reachable anyway.

1. MCP servers belong to the session. A session gets them from the plugin's `<plugin>/.mcp.json` or from the
   user's own configuration.
2. A tool from a plugin's server is named `mcp__plugin_vsdd-factory_<server>__<tool>`. The same tool from a
   server the user registered is named `mcp__<server>__<tool>`.
3. An agent with an explicit tool list gets an MCP tool only if the list names it.

**The design.** The plugin bundles four servers under fixed keys: `perplexity`, `context7`, `tavily` and
`playwright` (need N-PKG-1). Every agent that needs a tool lists both spellings, for example:

```
mcp__plugin_vsdd-factory_context7__resolve-library-id, mcp__context7__resolve-library-id,
mcp__plugin_vsdd-factory_context7__query-docs, mcp__context7__query-docs
```

Whichever server the session has, the name resolves; a name that does not resolve is dropped. This matters
because the platform drops the plugin's copy of a server when the user or the project already has the same one, so
the surviving tool name differs from machine to machine. A user who already
runs these servers (as this user does) keeps using them, and a user who has none enters the keys once in the
plugin's configuration. I chose "name both" over "bundle only" and "user registers only" because it needs no
setup in either case and no agent file changes when a user moves from one to the other.

**Who gets what** is the table in section 2.5. The principle: deep research tools (DEEP) are held only by
`researcher`; quick lookups (LOOKUP, DOCS, WEB) go to the roles that make technology or domain claims; builders
get library documentation (DOCS) so they do not rely on memory for an API; browser tools go to the three roles
that must look at a running product. An agent that needs more than its lookups returns a spike request.

**When a server is missing.** Nothing fails. The `research` method tells the researcher to prefer Perplexity,
Context7 and Tavily, to fall back to `WebSearch` and `WebFetch`, and to record which tools it used. Its result
carries `sources_count` and `tools_used`, so the workflow can refuse a report with no sources. Browser roles fall
back to the Playwright command line through `Bash`.

**Storybook** is a server that belongs to a project, not to the plugin. The `setup` skill tells the person to
register it under the name `storybook` when the project has a UI, and the four agents with the STORYBOOK set list
`mcp__storybook__<tool>`. Without it, the UI methods read Storybook's `index.json` instead.

**A limit of this choice.** A project's own MCP servers (a database, a design tool) do not reach builders,
because tools are listed explicitly. Section 7 records this as a risk.

**Why the names are pinned at all.** The packaging draft written alongside this document recommends that agents
do not pin MCP tool names in `tools`, because the names are not stable across machines. I pin them, in both
spellings, for two reasons. An agent with no `tools` list inherits every tool in the session, including the
user's unrelated servers, which is the fixed cost and the exposure that rule 3 exists to remove. And listing both
spellings is exactly what makes an unstable name harmless. Probe V3 in section 7 tests it; if it fails, the
fallback is that the agents with MCP sets, and only those, drop their `tools` list.

### 2.11 The spike rule

The spike agent is `vsdd-factory:spike-runner`. The spike command is `/vsdd-factory:spike`, which is the
pipeline design's `spike` workflow; there is no spike skill.

The rule agents carry is in the shared block of section 2.7. Stated once more:

- **If the agent has the Agent tool**, it dispatches `vsdd-factory:spike-runner` itself, with worktree
  isolation, and continues with the finding.
- **If it does not**, it is a workflow step. It stops and returns the spike request from its result schema. The
  workflow's shared wrapper runs the spike and re-runs the step with the finding.
- **A person** types `/vsdd-factory:spike <question>`, which runs the `spike` workflow. The main-session
  model, on its own judgment, dispatches the spike agent with the Agent tool: in the platform probe it never
  started a workflow unprompted, and it did dispatch the agent.

This improves on the reading in `rewrite/STATE.md` in three ways.

1. **The test is "do I have the Agent tool", not "am I in a workflow".** An agent cannot always tell where it is
   running, but it can always see its tools. The platform removes the Agent tool from workflow steps, so the test
   gives the right answer with no marker in the prompt.
2. **The spike runner records the ledger entry itself**, in all three entrances, by running `vsdd spike open`
   first and `vsdd spike record` last. The skill tested in the spike opened and closed the ledger from the
   main session; that left two entrances with no record.
3. **Desk research is a valid spike method.** A workflow step that needs deep research returns the same request,
   and `spike-runner` (which has web and lookup tools) chooses between an experiment, research, or both. One
   request type covers every "I need to know this before I continue".

**Every role can ask for a spike, and the runner inherits the asker's walls.** `code-reviewer`,
`holdout-evaluator` and `scenario-reviewer` are walled off from material a spike runner could otherwise read
(the specs for one, the source for the others), so a finding could carry that material back to them. The
user's requirement is that any agent can ask, so the wall moves with the request: `vsdd spike open` records
the asker's agent type in the ledger entry, and the hook runner denies that spike runner every class the
asker is denied, in addition to its own `holdout`, `reviews`, `runs` and everything outside the project.

The asker's type is never something an agent says. The first pass had the spike runner type it as an
argument, which a prompt-injected runner could change, and had the main session run a spike "on behalf of" a
walled reviewer, which recorded the main session as the asker. Both are gone. As a workflow step, an agent
returns a spike request; the hook has already stored that result with the agent's type, and the tool reads
the asker from that record. Dispatched in a normal session, an agent dispatches the spike runner itself with
the Agent tool, and the hook records who dispatched it. Every agent except the spike runner and the clerk
therefore lists the Agent tool, limited by the hook to the spike runner; `code-reviewer` and
`holdout-evaluator`, which the first pass left without it, have it now. An earlier finding for the same
question is returned only to an asker whose walls are no stricter than those the earlier runner worked
under.

`spike-runner` declares `isolation: worktree` in its frontmatter. The platform probe showed that this is
honoured for a dispatch with the Agent tool and ignored for a workflow step, so the workflow wrapper passes
isolation itself for every spike, research included, and the hook runner enforces the effect: a spike runner
with no worktree of its own can write nothing (need N-HOOK-3). Limits on how many spikes a step or a run may
start belong to the pipeline and hook designs. A spike needs an initialised project, and the `spike`
workflow takes no run lock (`pipeline.md` 2.8).

### 2.12 The skill roster

Thirteen skills. One, `status`, is model-visible. A workflow is already a slash command
(`/vsdd-factory:<workflow>`), so no skill exists to start one, with one deliberate exception: `drive`, which
follows the route from one workflow to the next so a person does not type each command.

| Skill | Purpose | Invoked by | What it does | Frontmatter beyond `name` and `description` |
|---|---|---|---|---|
| `status` | Shows the phase, work in flight, open spikes and health problems, the command that fixes each problem, and the exact next command. | person and model | Instructs the main session to run `vsdd status` with the given arguments and relay the output; when a person states a standing instruction, to save it with `vsdd note`. Its behaviour is the state design's status-and-resume entry point. Plain `status` writes nothing; `--fix` repairs (it releases a dead lock, removes expired worktrees, clones a missing member repository, commits pending state) and is the in-session form of every repair a message names. | `argument-hint: "[--fix]"` |
| `setup` | Makes a project a factory project, mounts state on a fresh clone, or prepares the import of pre-1.0 state (it runs `vsdd import --dry-run` and hands the person the import command, which only a person may run). | person | Instructs the main session, in this order. (1) Ask the decision D4 question before anything is written: may the plugin approve its own routine tool calls, and at which scope (`off`, `core`, or `extended`, each described; `extended` is recommended for unattended runs). With `--hub`, also run `vsdd repos scan` and show the member repositories for the person to confirm. (2) Run `vsdd init --auto-allow <scope>` (and `--hub` with the confirmed list). `init` is the one moment the plugin writes those keys, which is why the questions come first. On a project that is already initialised, show the person the lines to add instead. On pre-1.0 state `init` converts nothing: run `vsdd import --dry-run`, show the report, and give the person the `vsdd import --auto-allow <scope>` command. (3) Show the detected configuration, any tracked file that falls in no path class, and which lines of `factory.toml` to edit if a value is wrong (no tool call can write that file). (4) Run `vsdd doctor`. (5) Offer, each with the person's confirmation: the terminal shim; the setting that disables Claude Code's commit attribution; moving build conventions out of the project's `CLAUDE.md`, by dispatching the `architect` with the method `conventions`, because the main session may not write `conventions.md`; and moving the sections of `CLAUDE.md` that describe the old process to `docs/archive/CLAUDE-pre-1.0.md`, which the main session edits and the person commits. | `disable-model-invocation: true`, `argument-hint: "[--hub] [--repo]"` |
| `brief` | Guided creation of the product brief, with brainstorming techniques and a final check. | person | Instructs the main session. It is a conversation, so it cannot be a workflow. It saves `brief.md` as a draft after each section, so a compaction or a crash in the middle of the conversation loses nothing, and "the brief is in draft" is a route state whose next command is this skill. When the person says the draft is ready it dispatches the `product-owner` with the method `brief` to write the final text on the planner tier, and then runs `vsdd checkpoint brief`, which opens the cycle and the `brief` decision. | `disable-model-invocation: true`, `argument-hint: "[idea, or a path to notes]"` |
| `decide` | Settles a decision: the one act only a person may perform. | person | The hook runner records the decision from the typed command before any model reads it. The skill body tells the main session to relay the context the hook returned (the decision, the choice, the next command) and never to run `vsdd decide` itself. If that context says the driver was paused at this decision, the main session resumes the driver loop. | `disable-model-invocation: true`, `argument-hint: "<id> <choice> [key=value ...] [-- note]"` |
| `drive` | The low-touch driver: runs the pipeline from where it stands until a person is needed. | person | Instructs the main session to run `vsdd driver on`, read `next` from `vsdd status --json`, start that workflow with the Workflow tool, read `next` from its result and repeat. At a human decision it runs `vsdd driver pause <decision id>` and stops; when the person settles that decision, the `decide` hook's context tells the main session to resume, so the person does not type `/vsdd-factory:drive` again after each approval. At done, or after two runs with `progress: false`, it runs `vsdd driver off`. It lives in the main session because a workflow may nest only one level. | `disable-model-invocation: true` |
| `research` | Researches a topic with cited sources and saves a dated report. | person | Runs in a forked `researcher`. The researcher has no shell, so it cannot delete a report its new one replaces: its reply ends with the superseded paths, and the main session runs `vsdd retire <path>` for each. Inside a workflow the checkpoint does that from the step's `superseded` list. | `disable-model-invocation: true`, `context: fork`, `agent: vsdd-factory:researcher`, `argument-hint: "<domain or general or market> <topic>"` |
| `review` | An on-demand independent review outside the pipeline. | person | Instructs the main session to dispatch the reviewer for the lens with a fixed prompt (table below). | `disable-model-invocation: true`, `argument-hint: "<lens> [path or story]"` |
| `author` | Has the owning specialist create or change one artifact. | person | Instructs the main session to dispatch the owner agent with its method (table below). | `disable-model-invocation: true`, `argument-hint: "<artifact type> <what>"` |
| `retro` | A retrospective of a run: cost, timing, gate outcomes, wall integrity, what was learned, improvement proposals. | person | Runs in a forked `researcher` with the `retrospective` method. The report is a research report of kind `retrospective`; there is no lessons register. Each proposal names the line it would add to `conventions.md` or `review-rules.md`, and the reply tells the person the command that adopts it: `/vsdd-factory:author conventions <proposal>` or `author review-rules <proposal>`. | `disable-model-invocation: true`, `context: fork`, `agent: vsdd-factory:researcher`, `argument-hint: "[last run or wave N or release]"` |
| `defer` | Adds, lists or resolves an entry in the deferred-work register. | person | Instructs the main session: `add` runs `vsdd defer add` and writes the entry; `list` runs `vsdd defer list`; `resolve` runs `vsdd retire <path>`, the one command that deletes a file in a class that allows deletion. | `disable-model-invocation: true`, `argument-hint: "<add or list or resolve> ..."` |
| `observe` | Starts, stops and checks the local observability stack, registers a project with it, and sets the telemetry environment. | person | Instructs the main session to run the packaging design's observability command (`vsdd obs`). | `disable-model-invocation: true`, `argument-hint: "<up or down or status or register or telemetry>"` |
| `preview` | Starts or stops the local preview server that shows mockups and diagrams in a browser while designing. | person | Instructs the main session to run the server shipped in `<plugin>/skills/preview/app/`. The app keeps its `package.json` there, not at the plugin root, and installs its dependencies into the plugin's data directory on first use. | `disable-model-invocation: true`, `argument-hint: "<start or stop>"` |
| `export-pdf` | Converts a Markdown report to PDF with a neutral stylesheet, or one the project supplies. | person | Instructs the main session to run `vsdd export pdf`, which the packaging design provides. | `disable-model-invocation: true`, `argument-hint: "<file.md> [--css path]"` |

Because `defer` cannot be invoked by the model, the old rule that only a person may defer work is enforced by
the platform and not by a paragraph of prose.

**Lenses of `review`.** The skill first runs `vsdd review prepare <lens> [base]`, one command that writes the
diff patch (for `code` and `diff`) and returns the next free report path to hand the reviewer. The reviewer's
type decides which walls apply; when it needs a spike it dispatches the spike runner itself.

| Lens | Agent | Method |
|---|---|---|
| `specs` | `adversary` | `review-specs` |
| `stories` | `adversary` | `review-stories` |
| `code <story>` | `adversary` | `review-story-diff` |
| `system` (includes spec drift) | `adversary` | `review-system` |
| `extraction` | `adversary` | `review-extraction` |
| `diff [base]` | `code-reviewer` | `review-diff` |
| `security` | `security-reviewer` | `security-review` |
| `ui` | `ui-reviewer` | `ui-review` |

**Artifact types of `author`.**

| Type | Agent | Method |
|---|---|---|
| `decision` | `architect` | `decision-record` |
| `architecture`, `diagram`, `data-model` | `architect` | the method of the same name |
| `conventions`, `review-rules` | `architect` | `conventions`. This is how a retrospective's proposal is adopted, and how `setup` moves build conventions out of a project's `CLAUDE.md`: the edit lands in the file every builder (`conventions.md`) or every reviewer (`review-rules.md`) is given. These two guidance files are locked with the specification while a cycle is open, so nobody changes the rules in the middle of a build, and are writable by the architect between cycles, which is when a retrospective's proposals arrive. They are outside the `spec` decision's digest, so adopting one leaves the approval current. Any other specification file that is locked is changed through `/vsdd-factory:feature`, and the hook's block message says so |
| `contract`, `requirements`, `domain` | `product-owner` | `requirements`, `spec-delta` or `domain-spec` |
| `holdout` | `product-owner` | `holdout-scenarios` |
| `story` | `story-writer` | `story` |
| `ux`, `design-system` | `ux-designer` | `ux-spec` or `design-system` |
| `doc` | `technical-writer` | `docs` |
| `conform <path>` | the owner of the path's class | the owner's method, to fix what the lint tool reported |

**Skeletons of the two shapes.**

```markdown
---
name: decide
description: Settle a factory decision that is waiting on you.
argument-hint: "<id> <choice> [key=value ...] [-- note]"
disable-model-invocation: true
---

The hook runner has already recorded this decision, or refused it and told the person why.
Do not run `vsdd decide`. Relay the recorded decision and the next command from the context you were given,
in at most three lines.
```

```markdown
---
name: research
description: Research a topic with cited sources and save a dated report.
argument-hint: "<domain or general or market> <topic>"
disable-model-invocation: true
context: fork
agent: vsdd-factory:researcher
---

Method: `research`, or `market-intel` when the first word is `market`.
Topic: $ARGUMENTS
Write the report to the research directory. Return its path and a five-line summary.
```

**Why only one model-visible skill.** The platform spike measured about fifteen cents of fixed context per
agent start, and noted that skill listings are part of it. The model needs to find one thing on its own:
where the project stands. A question that needs an experiment is answered by the `spike` workflow or by
dispatching the spike agent, neither of which needs a skill. Everything else is either started by a person or
is a workflow. Agents themselves are found through the agent listing, which is why the 20 descriptions are one
sentence each.

**The roster as data.** The packaging design keeps an expectation file, `tests/lint/roster.toml`, that turns the
model policy and the context budget into failing tests. These are its entries from this design.

```toml
[tiers]
planner  = { model = "fable",  effort = "xhigh" }
builder  = { model = "sonnet", effort = "xhigh" }
reviewer = { model = "opus",   effort = "xhigh" }
clerk    = { model = "sonnet", effort = "low" }

[agents]                        # name = tier
product-owner = "planner"
architect = "planner"
ux-designer = "planner"
story-writer = "planner"
codebase-analyzer = "planner"
researcher = "planner"
spike-runner = "planner"
test-writer = "builder"
implementer = "builder"
verifier = "builder"
demo-recorder = "builder"
technical-writer = "builder"
devops = "builder"
adversary = "reviewer"
code-reviewer = "reviewer"
security-reviewer = "reviewer"
holdout-evaluator = "reviewer"
ui-reviewer = "reviewer"
scenario-reviewer = "reviewer"
clerk = "clerk"

[skills]                        # name = who may invoke it
status = "both"
setup = "person"
brief = "person"
decide = "person"
drive = "person"
research = "person"
review = "person"
author = "person"
retro = "person"
defer = "person"
observe = "person"
preview = "person"
export-pdf = "person"

[workflows]                     # name = { calls = [...] }; the twelve names of pipeline.md 2.3
story = { calls = [] }
wave = { calls = ["story"] }    # and so on for feature, verify, maintain and multi; the others call nothing

[overrides]                     # the only steps that run off their agent's tier (decision D5)
"ingest/map" = "builder"
"ingest/analyze" = "builder"

[budgets]
model_visible_skills = 1
listed_description_chars = 4400  # 20 agent descriptions of at most 200, 1 skill description of at most 400
agent_file_lines = 70            # whole file: frontmatter plus a body of at most 60 lines
skill_file_lines = 150
method_file_lines = 150          # an addition this design asks for
```

**Workflow names.** The pipeline design owns these twelve; the mapping in section 3.2 uses them.

| Name | Covers |
|---|---|
| `ingest` | Brownfield codebase ingestion (the old Phase 0), including disposition and port analysis for reference repositories |
| `spec` | Spec crystallization with its adversarial review loop (Phase 1), and the hidden scenarios |
| `plan` | Story decomposition, the computed wave schedule, readiness (Phase 2) |
| `wave` and `story` | One wave with its gate; one story (Phase 3) |
| `verify` | Holdout evaluation, adversarial refinement, formal hardening and convergence (Phases 4 to 7) |
| `release` | Release and the retrospective |
| `feature` | Feature mode (the old F1 to F7); its fast routes cover bug fixes and trivial changes |
| `maintain`, `discover`, `multi` | Maintenance sweeps; discovery and market assessment; multi-repo coordination |
| `spike` | Research spikes |

## 3. Mapping from the old system

### 3.1 Agents (35)

| Old agent | Status | 1.0 home | Note |
|---|---|---|---|
| accessibility-auditor | merged | `ui-reviewer` | Lens `accessibility` of `ui-review`. |
| adversary | ported | `adversary` | 384 lines to about 40. Perimeters become three methods. The worktree-identity preflight is gone: the workflow passes absolute paths. |
| architect | ported | `architect` | Assessments become methods. |
| business-analyst | merged | `product-owner`, `researcher` | Domain spec to the first, market analysis to the second. |
| code-reviewer | merged | `code-reviewer` | Merged with pr-reviewer. |
| codebase-analyzer | ported | `codebase-analyzer` | Paths to another project removed. |
| consistency-validator | replaced | lint tool; `adversary` | Mechanical checks become code; semantic checks join spec review. |
| data-engineer | merged | `architect`, `implementer` | Schema design; migration implementation. |
| demo-recorder | ported | `demo-recorder` | Also runs end-to-end journeys. |
| devops-engineer | merged | `devops` | |
| dtu-validator | merged | `verifier` | Method `clone-fidelity`. |
| dx-engineer | merged | `devops`, `setup` skill | The LiteLLM and three-model-family checks are dropped; they tested a platform that is gone. |
| e2e-tester | merged | `test-writer`, `demo-recorder`, `ui-reviewer`, `verifier` | Authoring, journey runs, responsive checks, Core Web Vitals. |
| formal-verifier | merged | `verifier` | |
| github-ops | merged | `devops` | Relay removed. |
| holdout-evaluator | ported | `holdout-evaluator` | Returns raw scores; the gate is computed. |
| implementer | ported | `implementer` | 369 lines to about 45. `git rebase -i` removed. |
| orchestrator (10 files) | replaced | workflows | `HEARTBEAT.md` is dropped: it was an OpenClaw health checklist. |
| performance-engineer | merged | `verifier` | Method `performance`. |
| pr-manager | replaced | `story` workflow; `devops` | Its two dispatch targets that did not exist are moot. |
| pr-reviewer | merged | `code-reviewer` | |
| product-owner | ported | `product-owner` | |
| research-agent | ported | `researcher` | Renamed. |
| security-reviewer | ported | `security-reviewer` | Moves from Sonnet to the reviewer tier, per the user's policy. |
| session-reviewer | merged | `researcher`, `retro` skill | Method `retrospective`. |
| spec-reviewer | merged | `adversary` | Its implementability checks join `review-specs` and `review-stories`. |
| spec-steward | replaced | git history; lint tool; `feature` workflow | |
| state-manager | replaced | code | See section 2.4. |
| story-writer | ported | `story-writer` | |
| stub-architect | merged | `test-writer` | Method `failing-tests`, which includes the stubs the tests need. |
| technical-writer | ported | `technical-writer` | |
| test-writer | ported | `test-writer` | |
| ux-designer | ported | `ux-designer` | |
| validate-extraction | merged | `adversary` | Method `review-extraction`. |
| visual-reviewer | merged | `ui-reviewer` | Lens `visual`. |

### 3.2 Skills (127)

Dispositions: **kept** (a 1.0 skill does the same job, possibly renamed); **merged** (into the named 1.0 skill);
**workflow** (replaced by the named workflow, with the method file that carries its content); **code** (the
capability is provided by a tool, a hook or derived data, so no skill is needed); **dropped** (with the reason).
Totals: 6 kept, 33 merged, 64 workflow, 14 code, 10 dropped. Workflow names are the pipeline design's twelve.

| # | Old skill | Disposition | 1.0 home, or reason |
|---|---|---|---|
| 1 | activate | dropped | 1.0 needs no activation. It installed a per-platform `hooks.json` and made the orchestrator the main agent; both are gone. |
| 2 | adversarial-review | kept | `review`. The loops live in the workflows. |
| 3 | agent-file-review | code | The agent and skill lint in the plugin's test suite. |
| 4 | analytics-integration | workflow | `discover`; method `discovery`. |
| 5 | artifact-detection | merged | `status` reports what exists and names the entry command. Existing documents are not detected silently: they are passed to `spec` in its `sources` argument, and the `intake` stage converts them and records which stages they cover (`pipeline.md` 2.9.3). |
| 6 | brainstorming | merged | `brief` (supporting file `techniques.md`). |
| 7 | brownfield-ingest | workflow | `ingest`; methods `ingest-broad`, `ingest-deepen`, `ingest-synthesis`, `review-extraction`. |
| 8 | check-input-drift | code | Staleness check in the lint tool, shown by `status`. |
| 9 | check-state-health | merged | `status`. |
| 10 | claude-telemetry | merged | `observe`. |
| 11 | code-delivery | workflow | `story`; delivery is the `vsdd story deliver` command. |
| 12 | compact-state | merged | `status`. The status is computed from capped records each time, so there is nothing to compact. |
| 13 | competitive-monitoring | workflow | `discover`, `maintain`; method `discovery`. |
| 14 | conform-to-template | merged | `author conform`. |
| 15 | consistency-validation | code | Lint tool. Duplicate of validate-consistency. |
| 16 | convergence-check | workflow | `verify`; the rule is `converge()` in every workflow and the tool's verdict function, which is not a command. |
| 17 | convergence-tracking | code | Computed by the workflow from result fields. The LiteLLM cost source is gone. |
| 18 | create-adr | merged | `author decision`; also a step of `spec`. |
| 19 | create-architecture | workflow | `spec`; method `architecture`. |
| 20 | create-brief | merged | `brief`. |
| 21 | create-domain-spec | workflow | `spec`; method `domain-spec`. |
| 22 | create-excalidraw | merged | `author diagram`. |
| 23 | create-prd | workflow | `spec`; method `requirements`. |
| 24 | create-story | merged | `author story`; also a step of `plan`. |
| 25 | customer-feedback-ingestion | workflow | `discover`; method `discovery`. |
| 26 | deactivate | dropped | No activation exists to reverse. |
| 27 | decompose-stories | workflow | `plan`; methods `decompose` and `story`. Waves are computed by `vsdd plan schedule`; hidden scenarios are written in `spec` with method `holdout-scenarios`. |
| 28 | deliver-story | workflow | `story`. |
| 29 | demo-recording | workflow | `story`; method `demo`. Duplicate of record-demo. |
| 30 | design-drift-detection | workflow | `maintain`; `ui-review` lens `design drift`. |
| 31 | design-system-bootstrap | workflow | `spec`; method `design-system`. |
| 32 | discovery-engine | workflow | `discover`; method `discovery`. |
| 33 | disposition-pass | workflow | `ingest`; method `disposition`. The lens is the project's own brief, not one named project. |
| 34 | dtu-creation | workflow | `story`; method `service-clone`. |
| 35 | dtu-validate | workflow | `wave` (the fidelity check in its gate), `verify`, `maintain`; method `clone-fidelity`. |
| 36 | excalidraw-export | merged | `author diagram`. Its render script was missing from the old plugin. |
| 37 | factory-cycles-bootstrap | dropped | It migrated one old directory layout to another. Importing old projects is a question for the state design. |
| 38 | factory-dashboard | merged | `status`. |
| 39 | factory-health | merged | `status`, `setup`. |
| 40 | factory-lock | code | The state and hook designs' concurrency guard, shown by `status`. |
| 41 | factory-obs | merged | `observe`. |
| 42 | factory-unlock | code | The same guard. `status` shows a held lock and the command that releases it. |
| 43 | factory-worktree-health | merged | `status`. |
| 44 | feature-mode-scoping-rules | workflow | `feature`; method `delta-analysis`. |
| 45 | fix-pr-delivery | workflow | `story` with kind `fix`, created by `vsdd story new-fix`. |
| 46 | formal-verify | workflow | `verify` (`vsdd gate formal` and the baseline gate measure); harness stories with methods `proofs`, `fuzzing`, `mutation`, `security-scan`. |
| 47 | generate-pdf | kept | `export-pdf`. One company's branding is removed; its script and stylesheet were missing from the old plugin. |
| 48 | guided-brief-creation | merged | `brief`. |
| 49 | holdout-eval | workflow | The scenarios check in the `wave` gate and in `verify`; method `holdout-eval`. |
| 50 | implementation-readiness | workflow | `plan`: the plan gate (`vsdd gate plan`) and method `review-stories`. |
| 51 | intelligence-synthesis | workflow | `discover`; method `discovery`. |
| 52 | jira | workflow | `discover`, `maintain`: `vsdd discover collect` runs the tracker command configured under `[[discover.source]]` and the researcher reads the result. |
| 53 | maintenance-sweep | workflow | `maintain`. The old sweep procedures live in the methods its steps name: `docs-drift` (technical-writer), `pattern-sweep` (code-reviewer), `holdout-eval` and `review-scenarios` for scenario freshness (the evaluator re-runs, the scenario-reviewer judges staleness and coverage), `security-review` for dependency triage, `ui-review`, `risk-sweep` and `discovery`. The measured sweeps are one `vsdd gate sweep` job. |
| 54 | market-intelligence-assessment | workflow | `discover` in assess mode, whose result is part of the `brief` decision packet; method `market-intel`. It is not a gate of `spec` and is off by default for feature requests. On demand: `research market`. |
| 55 | mode-decision-guide | merged | `status` names the entry workflow for the project's situation. |
| 56 | model-routing | dropped | It described LiteLLM routing to other vendors, which never existed on Claude Code. Tiers are agent frontmatter. |
| 57 | multi-repo-health | workflow | `multi`; shown by `status`. |
| 58 | multi-repo-phase-0-synthesis | workflow | `multi`; method `multi-repo-synthesis`. |
| 59 | multi-variant-design | workflow | `spec`; method `design-variants`. |
| 60 | next-step | merged | `status`. |
| 61 | onboard-observability | merged | `observe`. |
| 62 | perf-check | workflow | `verify`, `maintain` (`vsdd gate perf` measures); benchmark harness stories with method `performance`. |
| 63 | phase-0-codebase-ingestion | workflow | `ingest`. |
| 64 | phase-1-prd-revision | workflow | `spec`; method `requirements`. |
| 65 | phase-1-spec-crystallization | workflow | `spec`. |
| 66 | phase-1d-adversarial-spec-review | workflow | `spec`; method `review-specs`. |
| 67 | phase-2-story-decomposition | workflow | `plan`. |
| 68 | phase-3-tdd-implementation | workflow | `wave`. |
| 69 | phase-4-holdout-evaluation | workflow | `verify` (scenarios check) and the `wave` gate. |
| 70 | phase-5-adversarial-refinement | workflow | `verify` (adversarial check); method `review-system`. |
| 71 | phase-6-formal-hardening | workflow | `verify` (baseline and formal checks). |
| 72 | phase-7-convergence | workflow | `verify` (the loop and its report) and the `release` decision. |
| 73 | phase-f1-delta-analysis | workflow | `feature`; method `delta-analysis`. |
| 74 | phase-f2-spec-evolution | workflow | `feature`; method `spec-delta`. |
| 75 | phase-f3-incremental-stories | workflow | `feature`; method `story`. |
| 76 | phase-f4-delta-implementation | workflow | `feature`, through `wave` and `story`. Its enforcement script was missing from the old plugin. |
| 77 | phase-f5-scoped-adversarial | workflow | `feature`; method `review-wave`. |
| 78 | phase-f6-targeted-hardening | workflow | `feature`; the `verifier` methods scoped to the change. |
| 79 | phase-f7-delta-convergence | workflow | `feature`. |
| 80 | planning-research | merged | `research`. |
| 81 | policy-add | code | Replaced by one file of project review rules that reviewers are given. Editing a file needs no skill. |
| 82 | policy-registry | code | The same. The twelve baseline policies policed index formats that no longer exist. |
| 83 | post-feature-validation | workflow | `discover`, `maintain`; method `discovery`. |
| 84 | pr-create | workflow | `story`; the pull request is opened by `vsdd story deliver`. |
| 85 | pr-review-triage | workflow | `story`. Triage is the `owner` field on each finding. |
| 86 | quick-dev-routing | workflow | The fast routes of `feature` (`trivial`, `bugfix`, `hotfix`), derived by `vsdd feature route`. |
| 87 | record-demo | workflow | `story`; method `demo`. |
| 88 | recover-state | merged | `status`. State is derived from artifacts and git, so there is nothing to reconstruct. |
| 89 | register-artifact | dropped | Indexes are derived, so there is nothing to register an artifact in. |
| 90 | rehydrate-wave | merged | `status`. |
| 91 | release | workflow | `release`. There is no `release` method: the tool's `vsdd release` command prepares and publishes, and the retrospective step uses `retrospective`. |
| 92 | relocate-artifact | dropped | Paths are checked when a file is written, so an artifact cannot land in the wrong place. |
| 93 | repo-initialization | merged | `setup`; method `repo-setup`. |
| 94 | research-cache-ops | merged | `research`. The researcher reads earlier reports first. |
| 95 | research | kept | `research`. |
| 96 | responsive-validation | workflow | `wave`, `verify`; `ui-review` lens `responsive`. |
| 97 | run-phase | dropped | It was an interpreter for `.lobster` files. Workflows execute themselves. |
| 98 | scaffold-claude-md | merged | `setup`; method `claude-md`. |
| 99 | sdk-generation | workflow | `story`; method `sdk-generation`. |
| 100 | semport-analyze | workflow | `ingest`; method `port-analysis`. |
| 101 | session-review | kept | `retro`. |
| 102 | setup-env | merged | `setup`. |
| 103 | spec-drift | merged | `review system`. |
| 104 | spec-versioning | code | Git history is the version record; the changelog is derived. |
| 105 | state-burst | dropped | It was a protocol for committing several bookkeeping files at once. Those files are gone. |
| 106 | state-update | code | Closure and decision records are written by code, and the status is computed from them. |
| 107 | storybook-mcp-integration | workflow | `story`; methods `ui-implementation`, `ui-tests`, `ui-review`. `setup` registers the server. |
| 108 | systematic-debugging | workflow | `story`, including stories of kind `fix`; method `fix-findings`. |
| 109 | toolchain-provisioning | merged | `setup`; method `toolchain`. Its configuration file was missing from the old plugin. |
| 110 | traceability-extension | workflow | `feature`; method `spec-delta`. |
| 111 | track-debt | kept | `defer`. |
| 112 | ui-completeness-check | workflow | `wave`, `verify`; `ui-review` lens `completeness`. |
| 113 | ui-quality-gate | workflow | `verify` (`vsdd gate ui` and `ui-review`). |
| 114 | ux-heuristic-evaluation | workflow | `spec`, `verify`; `ui-review` lens `heuristics`. |
| 115 | validate-brief | merged | `brief` (its final check). The person's `brief` decision is the gate before `spec`. |
| 116 | validate-consistency | code | Lint tool. |
| 117 | validate-template-compliance | code | Lint tool. |
| 118 | validate-workflow | dropped | It checked `.lobster` files. Workflow scripts are tested in the plugin's own suite. |
| 119 | visual-companion | kept | `preview`. |
| 120 | wave-gate | workflow | `wave`. |
| 121 | wave-handoff | merged | `status`. |
| 122 | wave-reset | merged | `status`. |
| 123 | wave-scheduling | code | `vsdd plan schedule`, run by `plan`. |
| 124 | wave-status | merged | `status`. |
| 125 | worktree-manage | code | `vsdd survey` makes story worktrees and `vsdd story deliver` removes them. Stale worktrees are shown by `status`. |
| 126 | wrap | merged | `status`. |
| 127 | writing-skills | dropped | It is guidance for people who change the plugin, not for people who use it. It moves to the contributor documentation. |

### 3.3 The thirteen principles of AGENT-SOUL.md

| Principle | Where it lives in 1.0 |
|---|---|
| 1 Spec supremacy | Shared block, first ground rule; write scopes keep spec changes with their owner. |
| 2 Verification-first architecture | `architect` methods `architecture` and `verification-properties`. |
| 3 Red before green | The hook design's tests-before-code rule; write scopes for `test-writer` and `implementer`. |
| 4 Adversarial integrity | `adversary` rules; read walls. "A different model family" becomes a different model and a different view. |
| 5 Silent failures | Shared block, second ground rule; `ran`, `not_reviewed` and `not_run` in results. |
| 6 Convergence | Pipeline design. |
| 7 Full traceability | Lint tool and generated report (state design). |
| 8 Pragmatism over ceremony | A person decides at a human stop; `defer` is person-only. |
| 9 Cognitive diversity | Section 2.3: tiers and walls. Cross-vendor routing is not in 1.0. |
| 10 Errors are domain knowledge | `requirements` method (error taxonomy); `security-reviewer` returns CWE identifiers. |
| 11 Deliberate naivete | `architect` method `feasibility`; the methodology document. |
| 12 Compounding correctness | Fixed tiers in agent files; no fallback models. |
| 13 Filesystem as working memory | Shared block, third ground rule; state design. |

### 3.4 Sections of the old agent files

| Old section | 1.0 |
|---|---|
| Identity (with a nested fake frontmatter in nine files) | Dropped. The first paragraph says who the agent is. |
| Operating Procedure pointing to `../../FACTORY.md` | Dropped. The path did not resolve. |
| Contract (Inputs, Outputs, Success Criteria) | Kept as `Inputs` and `Result`. |
| Constraints (twice in five files) | Kept once, as `Rules`, at most eight. |
| Tool Access with OpenClaw profiles | Dropped. The `tools` frontmatter is the truth. |
| Failure and Escalation, three levels | Replaced by "When you cannot proceed" in the shared block. |
| Remember; the footer | Dropped. |
| Context Discipline lists | Dropped. The caller passes exact paths and walls are enforced. |
| Artifact Path Constraint | Dropped. The hook runner checks paths. |
| UI Quality Loop, Storybook access | Moved to methods `ui-implementation`, `ui-tests`, `ui-review`. |
| Information Asymmetry | Replaced by the tables in section 2.9. |
| MCP Tools | Replaced by the tool list and the `research` method. |
| Lessons Learned; Anchor Justification | Moved to method files where they still apply. Index-synchronisation lessons are dropped with the indexes. |
| Self-review and the four status words | Replaced by the common result fields. |

## 4. Interfaces I provide

| ID | To | What |
|---|---|---|
| P-1 | all | The 20 agent type names of section 2.4, exactly as `vsdd-factory:<name>`, with their tiers. |
| P-2 | pipeline | For each agent: what it must be given and the role fields it returns (section 2.6); the method names (section 2.8). |
| P-3 | pipeline | Reviewers and the holdout evaluator return raw findings and scores. They never return a pass or fail decision. |
| P-4 | pipeline | The spike agent `vsdd-factory:spike-runner`; the rule in section 2.11; the fact that the spike runner records its own ledger entry through `vsdd spike open` and `vsdd spike record`. The spike command is the pipeline's workflow. |
| P-5 | hooks | The read walls, write scopes, dispatch table and proposed command scopes of section 2.9, keyed on `agent_type`, including the rows for the main session and for any other type, as the content of `walls.toml` in the format of `hooks.md` 2.7. The spike agent is `vsdd-factory:spike-runner`, its file sets `isolation: worktree`, and the spike command is `/vsdd-factory:spike`. |
| P-6 | state | Which agent type writes each path class (the write-scope table). Reviewers write a report only in an on-demand review; builders' record is their commit. No agent maintains an index, a count or any status. |
| P-7 | packaging | The layout `<plugin>/agents/<name>.md` (flat, 20 files), `<plugin>/skills/<name>/SKILL.md` (13), `<plugin>/methods/<name>.md` (flat), `<plugin>/assets/` (design-system starter, demo templates, scaffold files). The preview server lives in `<plugin>/skills/preview/app/`. |
| P-8 | packaging | The lint rules. Agent files: frontmatter keys only from {`name`, `description`, `model`, `effort`, `tools`, `color`, `isolation`, `omitClaudeMd`}; the first five required; `model` and `effort` equal the agent's tier in `roster.toml` (`xhigh` everywhere except the clerk tier's `low`); `omitClaudeMd: true` on every agent; reviewer files contain the severity table and the key rule byte for byte; body at most 60 lines and no line over 120 characters except `description` and `tools`; the shared block present byte for byte; sections in order; description at most 200 characters; every method named exists; every agent has a row in the hook runner's wall data and every row has an agent; forbidden words (OpenClaw, dark-factory, Corverax, LiteLLM, GPT, Gemini, T1 to T3, `apply_patch`, `../../`). Skill files: exactly one without `disable-model-invocation: true`, and it is `status`; description at most 400 characters for it; only documented frontmatter keys; `SKILL.md` at most 150 lines; a forked skill names an existing agent; no skill has a workflow's name. Method files: at most 150 lines. |
| P-9 | packaging | Reserved skill names: `status`, `setup`, `brief`, `decide`, `drive`, `research`, `review`, `author`, `retro`, `defer`, `observe`, `preview`, `export-pdf`. The initialise skill is `setup`, the status skill is `status`, the decision command is `decide`, and the driver is `drive`. `spike` is a workflow name. |
| P-10 | packaging | The MCP tool names agents list, in both spellings (section 2.10), which the agent lint must accept as known tools; the bundled server list (Perplexity, Tavily, Context7, Playwright); and the probes of section 7. |
| P-11 | packaging | The roster as data, in the format of `tests/lint/roster.toml`, with the budgets (section 2.12). |

## 5. Interfaces I need

**From the pipeline design.**

| ID | Need |
|---|---|
| N-PIPE-1 | The final workflow names. None may equal a name in P-9. Each "assumed name" in section 2.12 must have a workflow that covers it. |
| N-PIPE-2 | Every step names an agent type from P-1. No step runs without an agent type; steps that run one command use `clerk`. Steps set `model` and `effort` from the prelude tables that the build generates from `roster.toml`. The only overrides are those the roster lists under `[overrides]` (`ingest/map` and `ingest/analyze` on the builder tier), which `step()` reads from the generated `OVERRIDE` table; a script cannot mark one of its own. |
| N-PIPE-3 | Step prompts name a method from section 2.8 and give absolute paths. They do not restate the procedure. Reviewer prompts follow the three rules at the end of section 2.9. |
| N-PIPE-4 | The shared result schemas contain: a status with values for done, blocked and spike needed; a summary; an artifact list; a spike request with at least the question, why it blocks, a suggested method and a time box; and the role fields of section 2.6 under the names given there. If a schema has a verdict field, the script computes it. |
| N-PIPE-5 | The shared wrapper runs `vsdd-factory:spike-runner` with worktree isolation for a spike request and re-runs the step with the finding. |
| N-PIPE-6 | The holdout gate (mean, minimum for must-pass scenarios, spread) is computed by the tool and the script from `scores`, as findings of category `holdout` with no location. Fixers receive `symptoms` and the cited contracts only, never scenario text. |

**From the state design.**

| ID | Need |
|---|---|
| N-STATE-1 | A glob for every path class in section 2.9, and the configuration keys `paths.source`, `paths.tests`, `paths.docs`, `paths.ci`. Each class in the state directory must be a directory no other class shares (apart from `brief`, one file inside `spec-product`), so that write scopes can be checked by path. |
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
| N-PKG-2 | A plugin CLI on the `Bash` path, named `vsdd`. Settled by the blueprint (section 4.9): the state design owns the pipeline-facing commands. The three things this design asked for exist as `vsdd gate <name>` (runs the project's gate commands and writes the result to `evidence`; it is the only such command), `vsdd review diff` and the diff patch the survey and the green gate write, and `vsdd review path <lens>` for an on-demand review (N-STATE-3). There is no `vsdd worktree`, `vsdd pr-merge` or `vsdd cache`. `vsdd validate` must also cover identifiers that do not resolve and traces that are missing, which the old consistency skills checked. |
| N-PKG-3 | The lint rules of P-8 in the test suite, and the probes of section 7 run before the agent files are written. |
| N-PKG-4 | The tools three skills drive: the observability command (`observe`; `vsdd obs` in the packaging draft), a Markdown-to-PDF converter (`export-pdf`), and a diagram renderer (`author diagram`). The last two were missing from the old plugin and are not in the packaging draft. The contributor documentation receives the content of the old `writing-skills`. |

## 6. Open questions for the user

These questions are carried into section 2 of the blueprint as decisions D5 (tiers and aliases), D6 (the main
session's wall) and D13 (servers).

1. **Tiers for three roles the policy does not name.** `codebase-analyzer` and `spike-runner` are on the planner
   tier because their output redirects everything after it, and `verifier` is on the builder tier because its
   tools, not its judgment, give the verdict. Brownfield ingestion runs many analyzer passes, so the planner tier
   there is the largest single cost in this design. Recommendation: accept as designed, and let the pipeline run
   the broad inventory passes of ingestion on the builder tier as a marked override.
2. **Model aliases or pinned model IDs.** Aliases (`fable`, `sonnet`, `opus`) follow each family's newest model
   with no upkeep, but behaviour can shift when an alias moves. Recommendation: aliases, with the end-to-end proof
   re-run as the check after any model release.
3. **Should the main session be walled like an agent?** As designed, the main session cannot read holdout
   scenarios and cannot edit spec artifacts directly; a person edits files in an editor or runs
   `/vsdd-factory:author`. This closes the last unenforced path around the walls, at the price of some friction.
   Recommendation: yes, on by default, with a project configuration switch.
4. **Research servers: bundled, your own, or both.** You already run Perplexity, Context7, Tavily and Playwright
   servers. The design accepts either yours or the plugin's. Bundled servers need a one-time approval in each
   project where the plugin is installed at project scope. Recommendation: bundle all four so a new machine needs
   only the keys, and leave the keys blank on machines that already have the servers.

## 7. Risks

1. **Platform behaviour this design assumes and has not verified.** Each is a one-line test in a throwaway
   plugin, like the first platform spike. They should run before the agent files are written.

   | Probe | Assumption | If it fails |
   |---|---|---|
   | V1 | `${CLAUDE_PLUGIN_ROOT}` is substituted in an agent file's body. | The hook runner adds the plugin root to the agent's context at `SubagentStart`, or the workflow wrapper puts it in every step prompt. |
   | V2 | A workflow step whose agent has an explicit tool list can still return a structured result. | Covered already by listing `StructuredOutput`; if the name differs, lint updates every file. |
   | V3 | An agent that lists an MCP tool can call it as a workflow step, by direct dispatch and as a forked skill; an unresolved name is dropped silently; no tool-search step is needed first. | Add the tool-search tool to the agents that have MCP sets. If that is not enough, those agents, and only those, drop their `tools` list and inherit. |
   | V4 | An agent without the Skill tool is not sent the skill listing. | No change in behaviour; the saving is smaller. |
   | V5 | `isolation: worktree` in a plugin agent's frontmatter is honoured, and the hook can see the isolation argument of an `Agent` call. | Callers pass it; the hook checks the spike's working directory instead. |
   | V6 | A forked skill can name a plugin agent as `vsdd-factory:<name>`. | Use the bare name, or switch the two forked skills to main-session dispatch. |
   | V7 | `omitClaudeMd` works for a plugin agent. | The holdout evaluator's wall is extended to CLAUDE.md by the hook. |

2. **Same-family review.** Opus reviewing Sonnet's work shares more blind spots than a different vendor would.
   The design compensates with walls, fresh context, hidden scenarios and tool-based verification, not with a
   second vendor. If review quality disappoints in the end-to-end proof, a second vendor is a new requirement,
   not a setting.
3. **Walls have edges.** A wall keyed on agent type is only as good as its coverage of `Bash`, of recursive
   search and of browser `file:` addresses, and a workflow author can still leak by writing a builder's summary
   into a reviewer's prompt. Needs N-HOOK-1, N-HOOK-9, N-STATE-8 and N-PIPE-3 exist for this, and the prompt rules
   need a test in the workflow suite.
4. **Merged agents can become grab-bags.** `devops` has ten methods. The split rule in section 2.4 is the
   control; the lint limit on agent file length is the alarm.
5. **A method file may not be read.** An agent could act without reading the method its task names. The
   control is that results carry fields only the method defines (the score scale, `novelty`, `tools_used`), so a
   skipped method shows up as a failed schema or a failed gate.
6. **Explicit tool lists keep project tools out.** A project's own MCP servers do not reach builders in 1.0.
   If this hurts, the fix is a project configuration list of extra tool names stamped into builder files at setup,
   which this design leaves out to keep agent files static.
7. **Untrusted content.** `researcher` and `spike-runner` read web pages. The researcher has no shell and a
   narrow write scope. The spike runner has a shell, confined to its worktree and barred from pushing, and
   depends on the hook design's secret and destructive-command guards.
8. **Refusals in security work.** The current models can decline a request their safety classifiers flag, and
   security review, fuzzing and exploit analysis are near that line. A step that returns no result must count as
   a failed step and reach a person, never as a clean review.
9. **Cost of the planner tier.** Seven agents run on the most expensive model at the highest effort the user
   chose. The tier is one line in each file, so it can be tuned after the end-to-end proof measures it.
10. **Name collisions.** Skills and workflows share the `/vsdd-factory:` namespace. P-9 and the lint rule guard
    it, but only once the pipeline's names are final.
11. **On-demand reviews pass through the main session.** The `review` skill dispatches from the main session,
    which could add its own framing to the prompt. Pipeline reviews are not affected, because scripts compose
    those prompts. If it matters, `review` splits into one forked skill per reviewer at no listing cost.
