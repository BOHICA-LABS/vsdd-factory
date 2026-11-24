<!-- Inventory of the 35 pre-rewrite agent definitions (plugins/vsdd-factory/agents at 1.0.0-rc.25). Captured 2026-11-23 by a read-only survey of branch develop @ 150bc36c. Historical reference for the 1.0 rewrite; not maintained. -->

I've inventoried all 35 entries (34 single-file agents plus the `orchestrator/` directory, 44 files and 11,487 lines). The biggest findings:

- **Model labels don't match the text.** Several reviewers have `model: opus` or `model: sonnet` in frontmatter, but their bodies say they run on Gemini or GPT-5.4.
- **Many shared relative paths don't resolve.** `../../FACTORY.md` and `../../templates/` are wrong from `agents/`.
- **Two dispatch targets in `pr-manager` don't exist** as agents.
- **About 30 files still describe tools with OpenClaw names** (`apply_patch`, `exec`, `process`, `agents_list`, and so on).

Paths below are relative to `/Users/jmagady/dev2/vsdd-factory/plugins/vsdd-factory/agents/` unless they start with `/`.

## A. Per-agent table

No file has an `effort` or `disallowedTools` field. `tools:` appears in only 5 files. Everything else inherits all tools.

| Agent | File | Lines | Frontmatter (verbatim) | Role | Shape |
|---|---|---|---|---|---|
| accessibility-auditor | accessibility-auditor.md | 287 | model: sonnet, color: red | WCAG AA/AAA audits at any checkpoint | file |
| adversary | adversary.md | 384 | tools: Read, Grep, Glob; model: opus; color: red | Fresh-context adversarial review of specs and code (3 perimeters) | file |
| architect | architect.md | 471 | model: sonnet, color: green | Architecture, ADRs, verification-ready design, DTU and gene-transfusion assessments | file |
| business-analyst | business-analyst.md | 201 | model: sonnet, color: blue | Writes the sharded L2 Domain Spec from brief and research | file |
| code-reviewer | code-reviewer.md | 157 | model: sonnet, color: red | Constructive code review "from a different model family" | file |
| codebase-analyzer | codebase-analyzer.md | 440 | model: opus; tools: Read, Write, Edit, Glob, Grep, Bash, NotebookEdit, WebFetch, WebSearch; color: green | Deep multi-pass codebase scan (brownfield-ingest, semport-analyze) | file |
| consistency-validator | consistency-validator.md | 367 | model: sonnet, color: red | Cross-document consistency and drift (80 criteria) | file |
| data-engineer | data-engineer.md | 124 | model: sonnet, color: green | Schemas and reversible migrations; keeps the pure-core boundary | file |
| demo-recorder | demo-recorder.md | 169 | model: sonnet, color: blue | VHS / Playwright demo evidence | file |
| devops-engineer | devops-engineer.md | 498 | model: sonnet, color: yellow | Repo init, CI/CD, worktrees, rebase gates, release | file |
| dtu-validator | dtu-validator.md | 158 | model: sonnet, color: red | Fidelity scoring of Digital Twin clones | file |
| dx-engineer | dx-engineer.md | 235 | model: sonnet, color: blue | Tool install, direnv, key-name checks, LLM/MCP preflight | file |
| e2e-tester | e2e-tester.md | 240 | model: sonnet, color: blue | E2E journeys, Playwright/Cypress, responsive checks, Core Web Vitals | file |
| formal-verifier | formal-verifier.md | 252 | model: opus, color: red | Kani proofs, fuzzing, mutation testing, security scans | file |
| github-ops | github-ops.md | 127 | model: sonnet, color: yellow | Runs `gh` commands for agents without shell access; makes no decisions | file |
| holdout-evaluator | holdout-evaluator.md | 110 | tools: Bash, Read; model: opus; color: red | Black-box evaluation against hidden scenarios | file |
| implementer | implementer.md | 369 | model: sonnet, color: green | Strict TDD implementation with micro-commits | file |
| orchestrator | orchestrator/orchestrator.md + 9 sub-files | 484 (1,785 total) | name, description only; no model or color | Pipeline driver | **directory** (§4) |
| performance-engineer | performance-engineer.md | 179 | model: sonnet, color: yellow | Benchmarks, regressions, Core Web Vitals and bundle budgets | file |
| pr-manager | pr-manager.md | 681 | model: sonnet, color: yellow | 9-step PR lifecycle coordinator | file |
| pr-reviewer | pr-reviewer.md | 205 | model: opus, color: red | Final diff-only PR review, posted via github-ops | file |
| product-owner | product-owner.md | 493 | model: sonnet, color: blue | L2 to L3 PRD with BC-S.SS.NNN contracts, holdout scenarios | file |
| research-agent | research-agent.md | 191 | tools: 11 MCP tools (4 Perplexity, 2 Context7, 5 Tavily) + Read, Write, Edit, Glob, Grep, WebSearch, WebFetch; model: opus; color: purple | External research with citations | file |
| security-reviewer | security-reviewer.md | 275 | model: sonnet, color: red | Manual security review and scan triage with CWE/CVE | file |
| session-reviewer | session-reviewer.md | 210 | model: sonnet, color: red | Post-pipeline analysis across 8 dimensions (description says "sidecar learning artifacts") | file |
| spec-reviewer | spec-reviewer.md | 200 | model: opus, color: red | Constructive post-adversary review of specs and stories | file |
| spec-steward | spec-steward.md | 240 | model: sonnet, color: yellow | Spec versioning, traceability, governance | file |
| state-manager | state-manager.md | 644 | model: sonnet, color: yellow | STATE.md and `.factory/` bookkeeping, factory-artifacts commits | file |
| story-writer | story-writer.md | 551 | model: sonnet, color: green | Splits specs into per-story files with AC-to-BC tracing | file |
| stub-architect | stub-architect.md | 207 | model: sonnet, color: yellow | Compilable `todo!()` stubs (Red Gate, BC-5.38.001) | file |
| technical-writer | technical-writer.md | 79 | model: sonnet, color: blue | Docs that describe current behavior only | file |
| test-writer | test-writer.md | 367 | model: sonnet, color: green | TDD test suites from BCs, UI state coverage | file |
| ux-designer | ux-designer.md | 222 | model: sonnet, color: blue | UX spec, wireframes, design system | file |
| validate-extraction | validate-extraction.md | 162 | tools: Read, Write, Edit, Grep, Glob, Bash; model: sonnet; color: red | Checks codebase-analyzer output against the code (max 3 rounds) | file |
| visual-reviewer | visual-reviewer.md | 207 | model: sonnet, color: red | Multimodal demo and screenshot review | file |

Colors: red 13, green 6, blue 7, yellow 7, purple 1.

## 1. Model assignment

- **`model: opus` (7):** adversary, codebase-analyzer, formal-verifier, holdout-evaluator, pr-reviewer, research-agent, spec-reviewer.
- **`model: sonnet` (27):** every other single-file agent.
- **No model field (10):** all files under `orchestrator/`.
- **No other values** anywhere.

Model claims in the body text that contradict the frontmatter:

- **spec-reviewer (opus):** says it uses Gemini, separate from the authors (Claude) and the adversary (GPT-5.4).
- **visual-reviewer (sonnet):** says "You (Gemini)" and uses "review-tier model".
- **code-reviewer (sonnet):** says it is a "DIFFERENT model family than the Builder (Claude)" and "review-tier model Pro".
- **session-reviewer (sonnet):** an embedded YAML block sets `model: adversary` and `tier: T1`.
- **adversary and pr-reviewer (opus):** both say "different model / model family".

Related config outside `agents/`:

- `/Users/jmagady/dev2/vsdd-factory/plugins/vsdd-factory/skills/model-routing/SKILL.md` defines LiteLLM tiers: judgment = Opus 4.6, implementation = Sonnet 4.6, validation = Haiku 4.5, adversary = GPT-5.4, review = Gemini 3.1 Pro, plus fallbacks. It says the holdout evaluator "MUST use adversary tier (GPT-5.4)".
- `docs/FACTORY.md` (line 402) says per-agent tiers live in `openclaw.json`.
- `.lobster` workflow steps carry `model_tier: adversary|review|builder`.
- dx-engineer's LLM check pings 4 models on `localhost:4000` but says "All 3 model families are REQUIRED".
- `per-story-delivery.md` "Model Selection" just says to use the least powerful model that works.

## 2. Shared boilerplate

| Section | Files | Typical size |
|---|---|---|
| `## Identity` (with `Agent ID: \`x\``, or a fake nested `---name/emoji/theme---` block in 9 files: demo-recorder, github-ops, dx-engineer, spec-reviewer, test-writer, pr-manager, state-manager, pr-reviewer, dtu-validator) | 29 | 3–12 lines |
| `## Operating Procedure` + `> **Global Operating Rules:** Read ../../FACTORY.md and ../../VSDD.md` + `> **Target Project:** Your working directory is the target project…` | 30 agents (+9 sequence files with the docs/ path) | 3 lines |
| A second H1 (`# X Agent`) after Operating Procedure, so most files have 2–3 H1s | most | — |
| `## Contract` with `### Inputs / ### Outputs / ### Success Criteria` | 26 | 15–30 lines (content varies) |
| `## Constraints` (NEVER / ALWAYS / MUST NOT list) | most; **twice** in business-analyst, demo-recorder, dtu-validator, e2e-tester, github-ops | 4–8 lines |
| `## Tool Access` with `- Profile: \`full\|coding\|read-only\|restricted\|minimal\`` and OpenClaw-style Available/Denied lists | 35 | 4–6 lines |
| `## Failure & Escalation` with Level 1 / 2 / 3 | 35 | 3–5 lines |
| `## Remember` (one bold line) | 35 | 2 lines |
| Footer `_Engine-wide principles: see ../docs/AGENT-SOUL.md_` | 33 (stub-architect and orchestrator use `../../docs/`) | 2 lines |
| `## Context Discipline` / `…Context Discipline (DF-021)` with Load / Do NOT load lists | 17 + 6 | 6–14 lines |
| `## Artifact Path Constraint` (points to `config/artifact-path-registry.yaml`) | 6: architect, business-analyst, product-owner, spec-steward, story-writer, technical-writer | 4 lines |
| `## UI Quality Loop Capabilities (DF-037)` + `## Storybook MCP Access (D18)` + `## Context Requirements` | 7 (5 with Storybook): accessibility-auditor, consistency-validator, e2e-tester, implementer, performance-engineer, ux-designer, visual-reviewer | 40–65 lines |
| `## Information Asymmetry (Wall)` | 9 | 5–20 lines |
| `## MCP Tools (Direct Access)` (Perplexity, then Context7, then Tavily) | 4: architect, business-analyst, product-owner, security-reviewer | ~16 lines |
| `## Anchor Justification Requirement` | 4 | 5–15 lines |
| `## Lessons Learned (apply to ALL projects)` | 3: adversary, implementer, story-writer | — |
| `## Before Reporting Back: Self-Review` + a DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED table | 3: implementer, test-writer, pr-manager | ~20 lines |
| "Iron Law" | only story-writer line 419 ("Full TDD Iron Law enforced") | — |

Estimated share of each file that is generic scaffolding (Identity, Operating Rules, Tool Access, Failure & Escalation, Remember, footer: about 20–30 lines):

- About 25–40% of small files (technical-writer, holdout-evaluator, github-ops, data-engineer).
- About 10–15% of mid-size files.
- About 5% of the largest (pr-manager, state-manager, story-writer).

Counting the DF-037 UI blocks and DF-021 context blocks too, the 7 UI agents are roughly 30–45% shared material. adversary, holdout-evaluator, codebase-analyzer, research-agent and validate-extraction have a different, leaner shape: no Identity or Operating Procedure, they start with `# Name` and a `## Templates` block using `${CLAUDE_PLUGIN_ROOT}`. session-reviewer also has no Identity section.

## 3. References outside `agents/`

**Workflows (`.lobster`)**
- `orchestrator.md` lists all 8 top-level and 8 phase workflows; all exist under `workflows/`.
- `brownfield-sequence.md` cites `workflows/skills/semport/SKILL.md`, which does **not exist** (neither does `skills/semport`; `skills/semport-analyze` does).
- Workflow `agent:` fields never name: stub-architect, story-writer, business-analyst, codebase-analyzer, validate-extraction, spec-steward, technical-writer, github-ops, session-reviewer, data-engineer.
- data-engineer and session-reviewer appear in no skill either; data-engineer is reachable only through the orchestrator routing table.

**Skills**
- The 23 `/vsdd-factory:` skill refs all exist (run-phase, next-step, validate-workflow, check-input-drift, systematic-debugging, dtu-creation, state-burst, …).
- Path refs: `skills/deliver-story`, `skills/mode-decision-guide`, `skills/state-burst`, `skills/dtu-creation`.
- Skills named in prose without the prefix: factory-worktree-health, repo-initialization, design-system-bootstrap, phase-0-codebase-ingestion, multi-repo-phase-0-synthesis, sdk-generation, code-delivery, fix-pr-delivery, adversarial-review, consistency-validation, register-artifact.

**`subagent_type` values**
- `vsdd-factory:github-ops` (18×), `state-manager`, `demo-recorder`, `implementer`, `test-writer`.
- **`vsdd-factory:security-review`** (pr-manager): no agent or skill has this name (the agent is `security-reviewer`).
- **`vsdd-factory:pr-review-triage`** (pr-manager): this is a skill, not an agent (the agent is `pr-reviewer`).

**Templates**
- About 70 distinct `templates/...` names; all exist in `plugins/vsdd-factory/templates/`.
- Path styles vary: 22 files use `../../templates/` (wrong from `agents/*.md`, correct only from `orchestrator/`), and 6 use `${CLAUDE_PLUGIN_ROOT}/templates/`.

**Rules, config, docs**
- `rules/lessons-codification.md` (adversary, orchestrator).
- `config/artifact-path-registry.yaml` (6 agents).
- `docs/FACTORY.md`, `docs/VSDD.md`, `docs/AGENT-SOUL.md`. "SOUL.md #4" is cited in adversary; accessibility-auditor has 4 SOUL.md mentions.

**bin/ tools**
- `bin/lobster-parse` (orchestrator).
- `bin/check-stale-verdict.sh` and `bin/enforce-merge-strategy.sh` (pr-manager).
- `bin/factory-lock-write.sh` (state-manager).
- `resolve-worktree-identity.sh` (adversary).
- `compute-input-hash` (product-owner).
- `cargo run -p last-amended-migrate` (state-manager).
- `.factory/hooks/verify-sha-currency.sh`, instantiated from a template (state-manager).

**Hooks** (all exist in `hooks/`, `hook-plugins/` or `hooks-registry.toml`)
- `validate-vp-consistency.sh` (architect).
- `pr-manager-completion-guard` SubagentStop (pr-manager).
- `stamp-state-timestamp` PostToolUse and the retired `verify-state-timestamp-refresh` (state-manager).
- "A hook blocks writes above 500 lines" (state-manager; matches `validate-state-size`).
- Information walls are said to be "enforced by Lobster context exclusion" in code-reviewer, security-reviewer, pr-reviewer and formal-verifier.

**`.factory/` paths**
- Used heavily: `STATE.md`, `specs/{prd.md, behavioral-contracts, verification-properties, architecture/ARCH-INDEX.md, domain-spec/, ux-spec/, research/}`, `stories/STORY-INDEX.md`, `cycles/<cycle>/{adversarial-reviews, burst-log, convergence-trajectory, lessons, <story-id>/adversary-convergence-state.json}`, `code-delivery/STORY-NNN/`, `holdout-scenarios/`, `design-system/`, `semport/`, `policies.yaml`, `merge-config.yaml`, `maintenance-config.yaml`, `discovery-config.yaml`, `.factory-project/` (multi-repo).
- Demo evidence lives in `docs/demo-evidence/<STORY-ID>/` per demo-recorder, but in `.factory/demo-evidence/` per visual-reviewer and session-reviewer.

**MCP servers**
- Perplexity (31 mentions), Context7 (23), Tavily (18), Storybook MCP (14), Playwright MCP, Excalidraw.

**Other model vendors**
- Gemini: spec-reviewer, visual-reviewer.
- GPT-5.4: spec-reviewer.
- "3 model families": dx-engineer, brownfield-sequence.
- "review-tier" and "adversary tier": code-reviewer, visual-reviewer, session-reviewer, dx-engineer.
- LiteLLM at `localhost:4000`: dx-engineer.

**OpenClaw / legacy vocabulary**
- **"OpenClaw"**: dtu-validator line 107 ("OpenClaw provider config"). Also `openclaw.json` in `docs/FACTORY.md`.
- **"dark-factory" / "Dark Factory"**: 16 files. orchestrator.md says workspace "Must NOT contain 'dark-factory'" and that agents' write tool "resolves… inside dark-factory".
- **OpenClaw tool names in the orchestrator**: `sessions_*`, `agents_list` (call on startup), `memory_*`, `web_*`, `agentId: "orchestrator"`, `runTimeoutSeconds` (default 7200).
- **HEARTBEAT.md**: `/subagents list`, `HEARTBEAT_OK`, "announce events".
- **Tier labels T1/T2/T3**: implementer, session-reviewer, ux-designer, e2e-tester, accessibility-auditor, visual-reviewer, performance-engineer, consistency-validator.
- **Tool profiles** `full / coding / read-only / restricted / minimal` with `apply_patch`, `exec`, `process`: about 30 files. session-reviewer adds `session_status`, `memory_search`, `memory_get`.
- **`mcporter` ("installed as ClawHub skill")**: dx-engineer (9×), steady-state, maintenance-sequence.
- **No `AGENTS.md`** references.
- **Hard-coded paths to another project**: codebase-analyzer has `/Users/jmagady/Dev/corverax/.reference/...` (about 12 lines) and "Corverax sandbox"; research-agent line 11 says "for the Corverax project".

## 4. The orchestrator directory

It has 10 files, all with `name` and `description` frontmatter and no model, color or tools:

| File | Lines | Contents |
|---|---|---|
| `orchestrator.md` | 484 | The agent itself (detailed below) |
| `greenfield-sequence.md` | 225 | Steps 1–5, then P1–P6, PP |
| `brownfield-sequence.md` | 133 | Phase 0 table (12 substeps), then hand-off to greenfield |
| `feature-sequence.md` | 168 | F1–F7, quick-dev and bug-fix routing |
| `per-story-delivery.md` | 266 | Per-story steps a–g, wave gate, Red Gate, main-checkout sync protocol |
| `steady-state.md` | 146 | Hand-off, hotfix, deprecation, notifications |
| `maintenance-sequence.md` | 111 | 11 parallel sweeps (Path 10) |
| `discovery-sequence.md` | 109 | Delphi scoring (Path 8) |
| `multi-repo.md` | 91 | Paths 3/4/10 + 11 |
| `HEARTBEAT.md` | 52 | 6-item health checklist |

The 9 sub-files describe themselves as "Not directly invokable", but because they carry `name:` frontmatter they may still be discovered as agents. I didn't check whether Claude Code scans subdirectories.

`orchestrator.md` is laid out as:

1. **Workflow Data:** points to `../../workflows/*.lobster`, parsed with `bin/lobster-parse` and jq, spawns each step's `agent` in `depends_on` order.
2. **Identity:** `# 🎯 Orchestrator`.
3. **Workspace Resolution.**
4. **Contract.**
5. **Constraints:** a mandatory-steps list, "state-manager LAST in every burst", split bursts over 8 artifacts, and product-owner before story-writer.
6. **Delegation:** a task preamble `cd <path> &&`, a 32-row Agent Routing Table (no stub-architect), and a 19-row Cross-Cutting Skills table.
7. **Operating Loop** (steps 0–8, with TaskCreate/TaskUpdate etc. loaded via ToolSearch).
8. **Policy Rubric Loading** from `.factory/policies.yaml`.
9. **VSDD Feedback Routing** table.
10. **Phase Sequence:** a gate table (Pre-Pipeline, Market Intel, Phases 0–7, Post-Pipeline).
11. **Structured Human Review Questions** template.
12. **Mode Detection:** `project.yaml` means multi-repo; `project-context.md` plus code means feature; `src/` means brownfield; otherwise greenfield; then discovery, maintenance and override.
13. **Reference Files** table (lists multi-repo twice).
14. **Pipeline Resume.**
15. **Index-First Gate Checks.**
16. **Tool Access.**
17. **Cycle-Closing Checklist** (`[process-gap]`).
18. **Failure & Escalation**, including transient-dispatch recovery.
19. **Sequences** link list.

The phase pipeline is described three times, in prose, in different places: the Phase Sequence table in `orchestrator.md`, the step-ID lists in each sequence file (S2-01… P1-01…P1-12, P2-01…, P3H-01, P4-01, P5-01, P6-01, PP-01…), and the `.lobster` files, which `orchestrator.md` says are the authoritative data.

## 5. Stale, contradictory or duplicated items

**Paths that don't resolve**
- 30 agents point to `../../FACTORY.md` / `../../VSDD.md`. From `agents/` that resolves to `plugins/FACTORY.md`, which doesn't exist; the real file is `plugins/vsdd-factory/docs/FACTORY.md`.
- `orchestrator.md` uses `../../FACTORY.md` (also missing), while its 9 sibling files use the correct `../../docs/FACTORY.md`.
- stub-architect's footer uses `../../docs/AGENT-SOUL.md`, which is wrong from `agents/`.

**Duplicated per-story playbook**
- `orchestrator/per-story-delivery.md` (266 lines) and `/Users/jmagady/dev2/vsdd-factory/plugins/vsdd-factory/workflows/phases/per-story-delivery.md` (228 lines) have different content.
- Both say they are authoritative over the `deliver-story` skill ("this file wins").

**Stub ownership**
- The orchestrator constraints and `per-story-delivery.md` both assign stubs to test-writer ("(Stub Architect)").
- A separate stub-architect agent exists but isn't in the routing table or any workflow `agent:` field.

**Phase-number drift**
- greenfield step IDs are shifted by one from their phases (Phase 4 uses P3H, Phase 5 uses P4, Phase 6 uses P5, Phase 7 uses P6).
- formal-verifier says to route back to "Phase 4"; session-reviewer says "adversarial review in Phase 4"; security-reviewer has "Phase 4 Finding Triage".
- feature-sequence says "per VSDD.md Phase 4" where greenfield says "Phase 6".
- `orchestrator.md` calls 7-D convergence both an "expansion of VSDD.md Phase 6" and "VSDD Phase 7 operationalized".
- brownfield says to run "Phase 1-6" under a heading that says "Phases 1-7".
- steady-state lists "Pipeline complete" under Phase 6.

**Counts that disagree**
- multi-repo says "9 maintenance sweeps" vs 11 in maintenance-sequence.
- "Spec coherence (33 criteria)" in steady-state and maintenance vs "Validation Criteria (80)" and "21+ rules" inside consistency-validator.
- discovery says "geometric mean across all 7 dimensions" but lists only 6.
- feature F7 says "max 10 cycles", then "After 5 cycles stalled".
- dx-engineer says "3 model families" but checks 4 models.
- codebase-analyzer's description says "6-pass", but the body has Passes 0–6 plus "Pass 8".

**pr-manager**
- Uses `security-review` and `pr-review-triage` as subagent_types, which don't match any agent.
- Contains facts specific to this repo: `gh api repos/drbothen/vsdd-factory`, `delete_branch_on_merge=true`, a Darwin-leg bats discipline, and the CLAUDE.md heredoc convention.
- "Stale-Verdict Detection" says the orchestrator runs `check-stale-verdict.sh`, while Step 8-pre-A has pr-manager run it through github-ops.
- A "READY verdict … covered_sha" section is addressed to pr-manager, but Step 8 reads `covered_sha` from pr-reviewer's verdict.
- The "Dependency-Ordered Merge" section (rebase, re-test) differs from the Step 7 dependency check.
- Relies on the Agent tool to spawn sub-agents from inside a sub-agent; pr-reviewer does the same.

**pr-reviewer**
- Constraints use BLOCKING / WARNING / NIT; the Finding Format uses blocking / suggestion / nit.
- Says it "CANNOT see `.factory/`" but writes to `.factory/code-delivery/.../pr-review.md`.

**state-manager**
- Says "NEVER run non-git shell commands (no `cargo`…)" but requires `cargo run -p last-amended-migrate` and `factory-lock-write.sh`.
- Has two `## Templates` sections with different path styles (lines 76 and 606).
- Its directory tree shows a single `domain-spec-L2.md` and `ux-spec.md`, while greenfield says "Do NOT produce a monolithic domain-spec-L2.md" (sharded `domain-spec/`, `ux-spec/` instead).
- Puts a `convergence_trajectory` YAML in STATE.md, contradicting its own rule that per-pass data goes in cycle files.

**product-owner**
- Says "CANNOT execute shell commands" but also says to run `compute-input-hash`.
- Its Primary Input is `domain-spec-L2.md`, while the same file elsewhere loads sharded `domain-spec/*.md`.

**session-reviewer**
- Frontmatter has no `tools` field, but the body claims read-only access (allow Read/Grep/Glob, deny Write/Edit/Bash/Agent).
- Reads `.factory/phase-*/…` paths (old layout; current layout uses `cycles/`).
- Its description doesn't match its body.

**Access claims that contradict Tool Access**
- accessibility-auditor and consistency-validator call themselves "T2 read-only", but their Tool Access is `full` with exec and `coding` with write respectively.
- visual-reviewer says "read-only" but its profile is `coding`.

**Adversarial-review location**
- spec-reviewer writes to `.factory/specs/adversarial-reviews/`.
- code-reviewer excludes `.factory/specs/adversarial-reviews/`, but its wall is defined as `.factory/cycles/**/adversarial-reviews/**`.

**Smaller items**
- implementer tells the agent to run `git rebase -i`, which is interactive.
- `orchestrator.md` mixes OpenClaw tools (`agents_list`, `sessions_*`) with Claude Code tools (Agent, `TaskCreate`, `ToolSearch`).
- HEARTBEAT is entirely OpenClaw-style.
- per-story-delivery and the Red Gate steps assume `cargo check` / `cargo test` (Rust only).
