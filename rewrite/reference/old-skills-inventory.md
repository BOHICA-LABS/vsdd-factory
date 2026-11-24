<!-- Inventory of the 127 pre-rewrite skills (plugins/vsdd-factory/skills at 1.0.0-rc.25). Captured 2026-11-23 by a read-only survey of branch develop @ 150bc36c. Historical reference for the 1.0 rewrite; not maintained. -->

I've inventoried all 127 skill directories (275 files: 127 SKILL.md plus 148 supporting files) under `/Users/jmagady/dev2/vsdd-factory/plugins/vsdd-factory/skills/`. Line counts come from `wc -l`, frontmatter from parsing every SKILL.md, and agent and skill links from grep. I read 17 central skills in full: run-phase, deliver-story, adversarial-review, wave-gate, brownfield-ingest, release, create-prd, rehydrate-wave, compact-state, phase-0, phase-3, phase-5, next-step, wave-status, multi-repo-health, research-cache-ops and the remaining phase-1/2/4/6/7 entry points. For the rest I read the frontmatter, the opening lines and the section outlines.

**Headline facts**
- **Frontmatter:** no skill sets `model:` or `effort:`.
- **Missing files:** five skills point at scripts, a stylesheet or a config file that don't exist in the plugin (section 5).
- **Legacy vocabulary:** OpenClaw, Dark Factory, Corverax, LiteLLM and GPT/Gemini/DeepSeek/Qwen routing appear in 16 skills (section 6).
- **Workflow coverage:** 71 skills are never referenced by any `.lobster` file (section 4).

## 1. Full skill table

**Frontmatter key.** Every skill has `name` and `description`, so only the extra keys are listed.
- `AT` = allowed-tools
- `AH` = argument-hint
- `DMI` = disable-model-invocation (`t` = true, `f` = false)
- `FORK:x` = `context: fork` plus `agent: x`
- `UI-f` = `user-invocable: false`
- `X` = non-standard keys this repo invented (`agents:{primary,supporting}`, `inputs`, `outputs`, plus `+gate` / `+cond` where present)

**Supporting files.** `steps(n)` means a `steps/` folder with n files; `_sc` is `_shared-context.md`. `0` means the folder holds only SKILL.md.

| Skill | Lines | Frontmatter extras | Description (first ~15 words) | Supporting files | Category |
|---|---|---|---|---|---|
| activate | 87 | DMI:t | Opt in to VSDD factory persona; detects platform, writes settings.local.json orchestrator default | 4: detect-platform.sh/.ps1, apply-platform.sh/.ps1 | Setup |
| adversarial-review | 217 | AH, DMI:f, FORK:adversary | Fresh-context adversarial review of specs or implementation; information asymmetry; min 3 clean passes | 0 | Adversarial |
| agent-file-review | 198 | – | Reviews AGENTS.md for compliance with Dark Factory agent design principles | 0 | Meta |
| analytics-integration | 192 | X | Reads product analytics to identify adoption, error patterns, usage signals | 0 | Discovery |
| artifact-detection | 175 | – | Scans for existing planning artifacts, validates, routes to correct pipeline entry | steps(5): 01–05 | Spec intake |
| brainstorming | 150 | – | Guided brainstorming (SCAMPER, reverse, mind-map, constraint removal) feeding brief | steps(6): 01–06 | Spec intake |
| brownfield-ingest | 463 | AH | Broad-then-converge codebase analysis; 6 broad passes then deepening to LOW novelty | steps(7): _sc, a–f | Brownfield |
| check-input-drift | 240 | AH | Scan .factory artifacts for input-hash drift; recompute hashes; report stale | 0 | Validation |
| check-state-health | 184 | AT | Diagnostic of STATE.md structure/size/phase numbering; HEALTHY/WARNINGS/NEEDS-COMPACT | 1: lib/check-autocompact-setting.sh | State |
| claude-telemetry | 156 | AH, AT | Wire Claude Code native OTel export into local observability stack (OTEL_* env) | 0 | Observability |
| code-delivery | 239 | – | Post-convergence delivery: push, PR with evidence, CI wait, merge by autonomy level | 0 | PR |
| compact-state | 154 | DMI:f, AT | Extract historical STATE.md content into cycle files; slim to <200 lines | 0 | State |
| competitive-monitoring | 174 | X | Monitors competitor releases, pricing, funding, acquisitions; update report | 0 | Discovery |
| conform-to-template | 147 | AH | Fix structural gaps in artifact by adding missing template fields/sections | 0 | Artifact hygiene |
| consistency-validation | 288 | – | Cross-document validation between PRD, architecture, UX, stories, implementation | 0 | Validation |
| convergence-check | 114 | AT | 7-dimension convergence validation (spec, tests, impl, verif, visual, perf, docs) | steps(8): _sc, a–g | Convergence |
| convergence-tracking | 164 | – | Quantitative convergence metrics across adversarial/mutation/formal runs | 0 | Convergence |
| create-adr | 233 | AH, AT | Scaffold ADR, allocate ID, ARCH-INDEX row, bidirectional supersede patch | 0 | Spec |
| create-architecture | 163 | AT | Sharded architecture docs from PRD + BCs; ADRs, diagrams, purity boundaries | 0 | Spec |
| create-brief | 132 | AT | Product brief via guided discovery; writes .factory/specs/product-brief.md | 0 | Spec |
| create-domain-spec | 130 | AT | Sharded L2 domain spec: entities, relationships, invariants, ubiquitous language | 0 | Spec |
| create-excalidraw | 185 | – | Generate .excalidraw JSON diagrams (architecture, ER, flow) | 0 | Design/diagram |
| create-prd | 156 | AT | PRD with behavioral contracts, error taxonomy, edge cases from brief + domain spec | 0 | Spec |
| create-story | 139 | AH, AT | Create/refine a single story spec with ACs, tasks, implementation details | 0 | Story |
| customer-feedback-ingestion | 238 | X | Ingest feedback (issues, tickets, reviews, Slack); categorize, dedupe, digest | 0 | Discovery |
| deactivate | 53 | DMI:t | Reverse activate: remove orchestrator default, platform state, hooks.json | 0 | Setup |
| decompose-stories | 200 | AT | PRD+arch into epics, stories, dependency graph, wave schedule | steps(6): _sc, a–e | Story |
| deliver-story | 244 | AH (with inline YAML comment), AT (+Task) | Deliver one story via TDD: test-writer→implementer→demo-recorder→pr-manager→devops | steps(9): _sc, a, b, c, d, d5, e, f, g | Story/TDD |
| demo-recording | 297 | – | Record demos (VHS/Playwright/cURL/harness) from ACs; WebM/GIF | 0 | Story/TDD |
| design-drift-detection | 114 | X+cond | Detect design-system drift in maintenance sweeps (Sweep 10) | 0 | Design/UX |
| design-system-bootstrap | 148 | X+gate+cond | Bootstrap design tokens/components (greenfield create, brownfield extract) | 0 | Design/UX |
| discovery-engine | 315 | – | Autonomous discovery of feature/product opportunities, scoring, routing | 0 | Discovery |
| disposition-pass | 170 | AH | Re-examine ingested repos through "Corverax vision lens": Model/Reimplement/Enhance/Leave | 0 | Brownfield |
| dtu-creation | 124 | – | Build behavioral clones of third-party services (Digital Twin Universe), Docker | 0 | Verification/DTU |
| dtu-validate | 123 | AT | DTU validation: clones of critical subsystems for regression detection | 0 | Verification/DTU |
| excalidraw-export | 51 | – | Batch render .excalidraw to PNG via Playwright; "Reference-only, not directly invokable" | 0 | Design/diagram |
| factory-cycles-bootstrap | 113 | AH | Migrate flat adversarial-review layout to cycle-keyed directories | 0 | State |
| factory-dashboard | 69 | AT (Bash) | Live markdown dashboard of phase, waves, hook activity, gates (read-only) | 0 | Diagnostics |
| factory-health | 245 | AT | Validate/auto-repair .factory worktree (orphan branch, mount, STATE.md) | 0 | Diagnostics |
| factory-lock | 80 | AT | Acquire cross-session lock on factory-artifacts (fetch-before-check, CAS push) | 0 | State/lock |
| factory-obs | 138 | AH, AT | Manage local OTel/Loki/Prometheus/Grafana stack | 0 | Observability |
| factory-unlock | 103 | AT | Release lock (self-held or --force with audit event) | 0 | State/lock |
| factory-worktree-health | 236 | – | Validate .factory/.factory-project worktrees at pipeline start; devops-engineer | 0 | Diagnostics |
| feature-mode-scoping-rules | 136 | – | Reference doc for scope determination in Feature Mode (F1–F7) | 0 | Feature mode (ref) |
| fix-pr-delivery | 188 | – | Streamlined fix-PR flow (FIX-P*-NNN); skips stubs/Red Gate/wave gate | 0 | PR |
| formal-verify | 174 | AT | Kani, cargo-fuzz, cargo-mutants, semgrep hardening gate | steps(5): _sc, a–d | Verification |
| generate-pdf | 115 | AT | "1898 & Co. branded" PDF from markdown research doc | 0 | Research output |
| guided-brief-creation | 172 | – | Facilitated staged elicitation from raw ideas to brief (BMAD-inspired) | steps(6): 01–06 | Spec intake |
| holdout-eval | 78 | AH, FORK:holdout-evaluator | Holdout evaluation against merged wave code; strict information asymmetry | 0 | Adversarial/holdout |
| implementation-readiness | 149 | – | Validate spec package (PRD+arch+stories) consistent and ready to build | 0 | Spec gate |
| intelligence-synthesis | 249 | X | Correlate market/feedback/analytics signals into scored insights | 0 | Discovery |
| jira | 147 | – | Reference docs for ankitpokhrel jira-cli; "Reference-only" | 0 | Setup/reference |
| maintenance-sweep | 289 | – | Periodic sweep: deps, doc drift, patterns, stale holdouts, perf; cleanup PRs | 0 | Maintenance |
| market-intelligence-assessment | 249 | X+gate | Mandatory pre-spec market intel; GO/CAUTION/STOP | 0 | Discovery |
| mode-decision-guide | 150 | – | Decision guide Greenfield vs Brownfield vs Feature | 0 | Orchestration (ref) |
| model-routing | 69 | – | Reference for LiteLLM model routing strategy and fallback chains | 0 | Meta (ref) |
| multi-repo-health | 22 | – | Scan .worktrees for multi-repo layout; wraps bin/multi-repo-scan | 0 | Brownfield/multi-repo |
| multi-repo-phase-0-synthesis | 164 | X+gate | Synthesize per-repo ingestion into unified project context | 0 | Brownfield/multi-repo |
| multi-variant-design | 87 | X+gate+cond | 2–3 design variants scored on 6 dimensions | 0 | Design/UX |
| next-step | 39 | – | Read STATE.md + workflow, propose next Lobster step (no execute) | 0 | Orchestration |
| onboard-observability | 147 | AT | One-command wire-up to obs stack (register + telemetry) | 0 | Observability |
| perf-check | 124 | AT | Benchmark regression, profiling, budget compliance | 0 | Verification |
| phase-0-codebase-ingestion | 45 | – | Phase 0 entry → brownfield-ingest; delegates to phase sub-workflow | 0 | Phase entry |
| phase-1-prd-revision | 59 | X+gate | PO revises PRD from architect feasibility review; max 3 iterations | 0 | Spec |
| phase-1-spec-crystallization | 49 | – | Phase 1 entry: brief→domain→PRD→BCs→architecture→VPs | 0 | Phase entry |
| phase-1d-adversarial-spec-review | 80 | – | Phase 1d: spec package to Adversary ("different model family") | steps(6): _sc, a–e | Adversarial |
| phase-2-story-decomposition | 45 | – | Phase 2 entry → decompose-stories | 0 | Phase entry |
| phase-3-tdd-implementation | 48 | – | Phase 3 entry → deliver-story | 0 | Phase entry |
| phase-4-holdout-evaluation | 39 | – | Phase 4 entry → holdout-eval ("different model family") | 0 | Phase entry |
| phase-5-adversarial-refinement | 37 | – | Phase 5 entry → adversarial-review implementation | 0 | Phase entry |
| phase-6-formal-hardening | 42 | – | Phase 6 entry → formal-verify | 0 | Phase entry |
| phase-7-convergence | 50 | – | Phase 7 entry → convergence-check → release | 0 | Phase entry |
| phase-f1-delta-analysis | 198 | – | F1: impact boundary, affected artifacts, regression risk | steps(7): 01–07 | Feature mode |
| phase-f2-spec-evolution | 184 | – | F2: incremental PRD/arch/VP delta | steps(8): 01–08 | Feature mode |
| phase-f3-incremental-stories | 147 | – | F3: new stories into dependency graph without cycles | steps(8): 01–08 | Feature mode |
| phase-f4-delta-implementation | 175 | – | F4: TDD on new stories + full regression | steps(8): 01–08 | Feature mode |
| phase-f5-scoped-adversarial | 186 | – | F5: adversarial on delta; "different model family" | steps(9): _sc, a–h | Feature mode |
| phase-f6-targeted-hardening | 171 | – | F6: formal/fuzz/mutation scoped to delta | steps(7): 01–07 | Feature mode |
| phase-f7-delta-convergence | 178 | – | F7: 5-dimension delta convergence + final human gate | steps(5): 01–05 | Feature mode |
| planning-research | 67 | – | Market/domain/technical research via Perplexity/Context7/Tavily | 0 | Research |
| policy-add | 91 | AH | Register governance policy in .factory/policies.yaml | 0 | Governance |
| policy-registry | 110 | AH | View/validate/manage policies.yaml | 0 | Governance |
| post-feature-validation | 212 | X | Post-ship feedback/analytics monitoring at intervals | 0 | Discovery |
| pr-create | 87 | AH, AT | PR with story context, mermaid, BC traceability targeting develop | 0 | PR |
| pr-review-triage | 89 | – | Classify pr-reviewer findings into fix routes (used by pr-manager) | 0 | PR |
| quick-dev-routing | 98 | – | Compressed Feature Mode for trivially-scoped changes | 0 | Feature mode |
| record-demo | 96 | AH, AT | Playwright demo evidence per AC | 0 | Story/TDD |
| recover-state | 164 | AH | Reconstruct STATE.md from on-disk artifacts | 0 | State |
| register-artifact | 146 | AH | Register new artifact (BC/VP/story/holdout) in its INDEX | 0 | Artifact hygiene |
| rehydrate-wave | 219 | AH (""), AT | Rehydrate after wave reset from wave-state.yaml via git show | 1: rehydrate-wave.sh | Wave/state |
| release | 317 | AH | Config-driven release: bump, CHANGELOG, tag, GH release, publish | 0 | Release |
| relocate-artifact | 311 | AH, AT | Find/move artifacts violating artifact-path-registry.yaml | 0 | Artifact hygiene |
| repo-initialization | 367 | – | Interactive repo creation (orchestrator + devops-engineer) | 0 | Setup |
| research-cache-ops | 38 | – | Wrap bin/research-cache for Perplexity/Context7 results | 0 | Research |
| research | 64 | AH, FORK:research-agent | External domain/tech research via research-agent | 0 | Research |
| responsive-validation | 127 | X+cond | Responsive screenshots at 4+ breakpoints | 0 | Design/UX |
| run-phase | 62 | AH | Execute a phase/mode .lobster: spawn declared agents in order | 0 | Orchestration |
| scaffold-claude-md | 144 | – | Auto-detect project, generate CLAUDE.md | 0 | Setup |
| sdk-generation | 383 | – | Multi-language SDKs from OpenAPI/protobuf/GraphQL | 0 | Delivery support |
| semport-analyze | 124 | AH | Semantic port: extract intent, design translation strategy | 0 | Brownfield |
| session-review | 157 | X (primary: `session-review`) | Post-pipeline review; improvement proposals | 0 | Diagnostics/retro |
| setup-env | 106 | AT | Validate/provision dev env, MCP health, lefthook/just/cargo | 0 | Setup |
| spec-drift | 80 | FORK:Explore | Compare implementation vs specs for drift | 0 | Validation |
| spec-versioning | 160 | – | Semver for specs, changelog, drift between code/spec versions | 0 | Spec |
| state-burst | 330 | DMI:f, AT | Single-Commit Burst Protocol (TD-VSDD-053) for state-manager bursts | 0 | State |
| state-update | 92 | UI-f, AT | Update STATE.md on phase transitions; internal | 0 | State |
| storybook-mcp-integration | 184 | X+cond | Storybook MCP as UI validation backbone | 0 | Design/UX |
| systematic-debugging | 142 | – | 4-phase root-cause debugging before fixes | 0 | Story/TDD support |
| toolchain-provisioning | 404 | – | Install verification toolchain per language | 0 | Setup |
| traceability-extension | 153 | – | Rules for appending traceability links in features | 0 | Feature mode (ref) |
| track-debt | 84 | AH, AT | Tech-debt register add/list/resolve | 0 | Bookkeeping |
| ui-completeness-check | 171 | X+gate+cond | UI traceability matrix; zero gaps before convergence | 0 | Design/UX |
| ui-quality-gate | 148 | X+cond | Comprehensive UI quality gate across 8 dimensions | 0 | Design/UX |
| ux-heuristic-evaluation | 163 | X+cond | Nielsen 10 heuristics + cognitive walkthrough | 0 | Design/UX |
| validate-brief | 137 | – | Validate brief structure/quality, gaps vs bloat | 0 | Spec gate |
| validate-consistency | 187 | AT | Cross-file IDs, anchors, counts, naming, traceability | 11 fixtures/ (bc-tv-consistency ×6, tautology ×5) | Validation |
| validate-template-compliance | 152 | AH | Read-only audit of artifacts vs templates | 0 | Validation |
| validate-workflow | 47 | AH | Schema-check a .lobster file via bin/lobster-parse | 0 | Meta/validation |
| visual-companion | 108 | – | Local Node/Vite server for live mockups/diagrams | 16 (Vite/React app: setup.sh, visual-guide.md, package*.json, vite.config.js, .gitignore, scripts/×5, src/×5) | Design/UX |
| wave-gate | 285 | AH, AT | Post-wave gate: tests, DTU, adversarial, demo, holdout, state; blocks next wave | 0 | Wave |
| wave-handoff | 158 | AH (""), AT | Write HANDOFF.md + wave-state.yaml atomically at wave close | 5: wave-handoff.sh, lib/{commit-to-artifacts, write-handoff, write-wave-state, parse-sprint-state}.sh | Wave |
| wave-reset | 146 | AH (""), AT | Wave-boundary reset; mandatory /rehydrate-wave first | 0 | Wave |
| wave-scheduling | 143 | – | Topo-sort stories into waves and parallel groups | 0 | Story |
| wave-status | 22 | – | Report wave readiness; wraps bin/wave-state | 0 | Wave |
| worktree-manage | 104 | AH, AT | Create/list/cleanup story worktrees | 0 | Story support |
| wrap | 414 | AT (Bash, Read, Skill, Agent) | Pause factory, make state durable, release lock, resume instructions | 0 | State |
| writing-skills | 171 | – | TDD for skill authoring (pressure scenarios first) | 0 | Meta |

**Frontmatter totals:**
- `allowed-tools` 36, `argument-hint` 31.
- `context: fork` / `agent:` 4: adversarial-review→adversary, holdout-eval→holdout-evaluator, research→research-agent, spec-drift→Explore.
- `disable-model-invocation` 5: true on activate and deactivate; false on adversarial-review, compact-state and state-burst.
- `user-invocable: false` 1 (state-update).
- Non-standard `agents/inputs/outputs` 17, `gate` 6, `condition` 8.
- Many frontmatters have a blank line before `allowed-tools`.

**Step-file naming:** two conventions coexist. Some skills use letters (`step-a…`) with `_shared-context.md`; others use numbers (`step-01…`).

## 2. Category breakdown (127)

- **Phase entry points / orchestration (13):** phase-0, phase-1-spec-crystallization, phase-2, phase-3, phase-4, phase-5, phase-6, phase-7, run-phase, next-step, mode-decision-guide, quick-dev-routing, artifact-detection.
- **Spec intake and authoring (15):** brainstorming, guided-brief-creation, create-brief, validate-brief, create-domain-spec, create-prd, phase-1-prd-revision, create-architecture, create-adr, spec-versioning, implementation-readiness, create-excalidraw, excalidraw-export, planning-research, research.
- **Story / TDD delivery (10):** decompose-stories, create-story, wave-scheduling, deliver-story, record-demo, demo-recording, worktree-manage, systematic-debugging, sdk-generation, dtu-creation.
- **Adversarial / holdout / convergence / verification (9):** adversarial-review, phase-1d-adversarial-spec-review, holdout-eval, convergence-check, convergence-tracking, formal-verify, perf-check, dtu-validate, spec-drift.
- **PR lifecycle (4):** code-delivery, fix-pr-delivery, pr-create, pr-review-triage.
- **Wave / state / cycle bookkeeping (17):** wave-gate, wave-handoff, wave-reset, rehydrate-wave, wave-status, compact-state, state-update, state-burst, recover-state, check-state-health, factory-cycles-bootstrap, factory-lock, factory-unlock, wrap, track-debt, policy-add, policy-registry.
- **Artifact hygiene / validation (8):** consistency-validation, validate-consistency, validate-template-compliance, conform-to-template, register-artifact, relocate-artifact, check-input-drift, validate-workflow.
- **Feature mode (10):** phase-f1 to phase-f7, feature-mode-scoping-rules, traceability-extension, maintenance-sweep.
- **Brownfield / multi-repo (5):** brownfield-ingest, semport-analyze, disposition-pass, multi-repo-phase-0-synthesis, multi-repo-health.
- **Release (1):** release.
- **Discovery / market research (9):** discovery-engine, market-intelligence-assessment, competitive-monitoring, customer-feedback-ingestion, analytics-integration, intelligence-synthesis, post-feature-validation, research-cache-ops, generate-pdf.
- **Design / UX (9):** design-system-bootstrap, design-drift-detection, multi-variant-design, responsive-validation, storybook-mcp-integration, ui-completeness-check, ui-quality-gate, ux-heuristic-evaluation, visual-companion.
- **Diagnostics / observability (7):** factory-health, factory-worktree-health, factory-dashboard, factory-obs, onboard-observability, claude-telemetry, session-review.
- **Setup / onboarding (7):** activate, deactivate, scaffold-claude-md, setup-env, toolchain-provisioning, repo-initialization, jira.
- **Meta (3):** writing-skills, agent-file-review, model-routing.

Total: 13+15+10+9+4+17+8+10+5+1+9+9+7+7+3 = 127.

## 3. How skills invoke agents and other skills

**Dispatch mechanisms in use**
- **`context: fork` + `agent:`** (4 skills, listed in section 1).
- **Task tool:** run-phase ("spawn the declared agent via the Task tool") and deliver-story (`allowed-tools` includes `Task`).
- **Agent tool with `subagent_type`:**
  - Only two places spell it out: brownfield-ingest (`subagent_type: codebase-analyzer`) and code-delivery (`Agent(subagent_type="vsdd-factory:github-ops", …)`, twice).
  - wrap lists `Agent` in `allowed-tools` and names `vsdd-factory:state-manager`.
- **Prose "spawn X" (orchestrator delegates):** 12 skills start with the same "Delegation Reference" banner: discovery-engine, guided-brief-creation, dtu-creation, repo-initialization, phase-1d and phase-f1 to f7.
- **Non-standard `agents:` frontmatter** (17 skills) only declares primary and supporting agents; nothing in the harness acts on it.

**Which agents each skill names (mention counts, 30 agent files exist)**
- **deliver-story** (including steps): test-writer 25, implementer 23, adversary 24, pr-manager 24, devops-engineer 15, demo-recorder 9, story-writer 6, github-ops 4, pr-reviewer 2, state-manager 2.
- **code-delivery / fix-pr-delivery:** pr-manager → github-ops (create and merge), pr-reviewer, security-reviewer; devops-engineer and state-manager for cleanup.
- **adversarial-review:** adversary, plus state-manager to write the findings file and the ADV index (the adversary is read-only).
- **phase-1d / phase-f5:** adversary (55 and 58 mentions), consistency-validator, implementer, security-reviewer.
- **brownfield-ingest:** codebase-analyzer, validate-extraction, a general-purpose agent for the B.5 coverage audit. semport-analyze uses validate-extraction.
- **wave-gate:** holdout-evaluator (via /holdout-eval), adversary (via /adversarial-review), demo-recorder.
- **Research-agent dispatchers:** research, planning-research, market-intelligence-assessment, discovery-engine, competitive-monitoring, customer-feedback-ingestion, guided-brief-creation, dtu-creation.
- **Lock and state:** factory-lock / factory-unlock / state-burst / wrap → state-manager.
- **Setup:** repo-initialization → architect, devops-engineer, dx-engineer. toolchain-provisioning → devops-engineer.
- **Name mismatches:**
  - session-review declares `primary: session-review`, but the agent file is `session-reviewer.md`.
  - spec-drift forks to the built-in `Explore` agent, not a plugin agent.

**Skill-to-skill calls (main edges)**
- **run-phase** → phase-0 to phase-7 lobsters, factory-health, next-step.
- **next-step** → run-phase, factory-health. **wave-status** → wave-gate, run-phase.
- **Phase entry points:**
  - phase-0 → brownfield-ingest
  - phase-1 → create-brief, create-domain-spec, create-prd, create-architecture
  - phase-1d → adversarial-review, factory-cycles-bootstrap
  - phase-2 → decompose-stories; phase-3 → deliver-story; phase-4 → holdout-eval; phase-5 → adversarial-review
  - phase-6 → formal-verify; phase-7 → convergence-check, release
- **Brief and spec chain:**
  - artifact-detection → brainstorming, guided-brief-creation, create-prd
  - guided-brief-creation → brainstorming, adversarial-review, create-prd, visual-companion, create-excalidraw, state-update
  - create-brief → create-domain-spec, create-prd, research, register-artifact, state-update, factory-health
  - create-prd → create-architecture, adversarial-review, research, register-artifact
  - create-architecture → adversarial-review, create-excalidraw, visual-companion, research, register-artifact
- **Stories and waves:**
  - decompose-stories → create-story, deliver-story, state-update; create-story → deliver-story
  - deliver-story → wave-gate
  - wave-gate → adversarial-review, record-demo, holdout-eval, deliver-story
  - wave-reset → wave-handoff, rehydrate-wave; rehydrate-wave → wave-handoff
- **State:**
  - wrap → check-state-health, compact-state, factory-unlock, next-step, recover-state, rehydrate-wave
  - compact-state → rehydrate-wave; check-state-health → compact-state, factory-health, rehydrate-wave
  - factory-health → compact-state, factory-lock, factory-worktree-health, onboard-observability
- **Brownfield:**
  - brownfield-ingest → disposition-pass, semport-analyze, create-brief, create-domain-spec, create-prd, create-architecture, decompose-stories
  - disposition-pass → the same downstream skills plus research
- **Convergence:** convergence-check → formal-verify, perf-check, spec-drift, release.
- **Feature mode:**
  - phase-f1 → feature-mode-scoping-rules, quick-dev-routing
  - phase-f2 → phase-f1, spec-versioning; phase-f3 → phase-f2; phase-f6 → phase-f1, phase-f2
  - phase-f5 → adversarial-review, code-delivery, phase-f1
- **PR / review:** pr-review-triage → code-delivery.
- **Artifact hygiene / observability:**
  - conform-to-template → validate-template-compliance, register-artifact; relocate-artifact → register-artifact
  - onboard-observability → factory-obs, claude-telemetry
  - policy-add → policy-registry; research-cache-ops → research; activate ↔ deactivate, activate → scaffold-claude-md

**Core pipeline call graph**
```
[greenfield.lobster | run-phase | orchestrator]
 └ artifact-detection → brainstorming / guided-brief-creation → validate-brief
 └ phase-1-spec-crystallization (lobster)
     create-brief → create-domain-spec → create-prd → create-architecture
     → phase-1-prd-revision (product-owner⇄architect) → phase-1d-adversarial-spec-review (adversary loop ≥3 clean)
 └ phase-2-story-decomposition → decompose-stories steps a–e (epics, stories, dep graph, waves, holdout scenarios) [create-story, wave-scheduling]
 └ phase-3-tdd-implementation → deliver-story steps a,b,c,d,d5,e,f,g (per story)
     devops-engineer(worktree) → test-writer(stubs) → test-writer(failing tests, Red Gate) → implementer(TDD)
     → adversary(D.5 per-story convergence) → demo-recorder → implementer(push)
     → pr-manager [code-delivery: github-ops create PR, pr-reviewer, security-reviewer, CI, merge; pr-review-triage]
     → devops-engineer(cleanup) → sprint-state update
 └ wave-gate (tests → DTU → /adversarial-review implementation → /record-demo → /holdout-eval → state; mutation for facade stories)
     → wave-handoff → wave-reset → rehydrate-wave
 └ phase-4 holdout-eval → phase-5 adversarial-review (+code-reviewer secondary) [fix-pr-delivery for FIX-P*]
 └ phase-6 formal-verify a–d → phase-7 convergence-check a–g (perf-check, spec-drift) → release
```

**Inconsistency inside deliver-story:** its SKILL.md lists 9 numbered steps and never mentions the adversary step. The `steps/step-d5-adversary-convergence.md` file and wave-gate's "Step 4.5 of per-story-delivery.md" both rely on it. phase-3's step table (A–G) also leaves out D5. There are two canonical-workflow files: `agents/orchestrator/per-story-delivery.md` (266 lines, which deliver-story calls authoritative) and `workflows/phases/per-story-delivery.md` (228 lines).

## 4. Links to workflows, bin/, templates/, rules/ and hooks

**Which skills the workflows invoke** (`skill: "skills/…"` in `.lobster` files)
- **Phase lobsters point straight at step files** for brownfield-ingest a–f, decompose-stories a–e, deliver-story a–g plus d5, formal-verify a–d, and convergence-check a–g. In those five skills the steps/ folder is the executable unit and SKILL.md is a parallel description, so the content exists twice.
- **Most-referenced from workflows:**
  - session-review ×7 (final step of every top-level workflow), phase-4-holdout-evaluation ×5, check-input-drift ×5
  - ui-quality-gate / ui-completeness-check / responsive-validation / demo-recording / market-intelligence-assessment ×4 each
  - all F-phase SKILL.md files; discovery skills in discovery.lobster
- **Never referenced by any workflow (71):**
  - activate, agent-file-review, check-state-health, claude-telemetry, code-delivery, compact-state, conform-to-template, consistency-validation, convergence-tracking, create-adr, create-excalidraw, create-story, deactivate, disposition-pass, dtu-creation, dtu-validate, excalidraw-export, factory-cycles-bootstrap, factory-dashboard, factory-health, factory-lock, factory-obs, factory-unlock, feature-mode-scoping-rules
  - generate-pdf, jira, maintenance-sweep, mode-decision-guide, model-routing, multi-repo-health, next-step, onboard-observability, perf-check, phase-3-tdd-implementation, phase-f4-delta-implementation, policy-add, policy-registry, pr-create, pr-review-triage, quick-dev-routing, record-demo, recover-state, register-artifact
  - rehydrate-wave, relocate-artifact, research, research-cache-ops, run-phase, setup-env, spec-drift, spec-versioning, state-burst, state-update, systematic-debugging, traceability-extension, track-debt, validate-consistency, validate-template-compliance, validate-workflow, visual-companion, wave-gate, wave-handoff, wave-reset, wave-scheduling, wave-status, worktree-manage, wrap, writing-skills
  - Note: code-delivery is described as "via code-delivery.lobster" in 13 places, but that workflow uses agent steps, not the skill.

**Thin wrappers**
- **Over a `.lobster` file:** phase-0 to phase-7 (37–50 lines each, all saying "Execute the steps defined in `${CLAUDE_PLUGIN_ROOT}/workflows/phases/<phase>.lobster`" and pointing to a "Work Skill").
- **Lobster runners:** run-phase, next-step, validate-workflow. They are generic interpreters over `bin/lobster-parse` and assume `type: agent|skill|command`, `depends_on`, `on_failure` and `max_retries` fields.
- **Over bin/ tools:**
  - wave-status → `bin/wave-state`
  - multi-repo-health → `bin/multi-repo-scan`
  - research-cache-ops → `bin/research-cache`
  - factory-dashboard → `bin/factory-dashboard`
  - factory-obs / onboard-observability → `bin/factory-obs`
  - check-input-drift → `bin/compute-input-hash`
  - adversarial-review and deliver-story → `bin/resolve-worktree-identity.sh`
  - factory-lock → `factory-lock-acquire-precheck.sh`, `factory-lock-write.sh`, `factory-cas-push.sh`, `emit-event`
  - factory-unlock → `factory-unlock-decide.sh`, `factory-lock-write.sh`, `factory-cas-push.sh`, `emit-event`
  - state-burst → `factory-cas-push.sh`, `factory-lock-write.sh`
  - factory-health / factory-worktree-health → `factory-lock-status.sh`
  - create-adr → `validate-template-compliance.sh`, `emit-event`
- **Over their own scripts:** rehydrate-wave (`rehydrate-wave.sh`), wave-handoff (`wave-handoff.sh` + lib/), activate (detect/apply-platform; apply calls `scripts/generate-hooks-json.sh`), visual-companion (`start-server.sh`, `stop-server.sh`).
- **bin/ tools no skill references:** check-stale-verdict.sh, enforce-merge-strategy.sh, factory-query, factory-replay, factory-report, factory-sla.

**templates/** (109 files): 51 skills cite `${CLAUDE_PLUGIN_ROOT}/templates/...`. The heaviest are create-prd (6), compact-state, decompose-stories and formal-verify (5 each), and create-architecture, holdout-eval and phase-1d (4 each).

**Artifact path registry:** 10 skills carry a "Path Resolution (Mandatory)" block pointing to `config/artifact-path-registry.yaml`: conform-to-template, create-adr, create-architecture, create-domain-spec, create-excalidraw, create-brief, create-prd, create-story, register-artifact, relocate-artifact.

**rules/:** barely cited.
- create-story → `.claude/rules/story-completeness.md` (a `.claude/` path, not the plugin's `rules/`).
- create-prd, create-architecture and decompose-stories → "`spec-format.md` rule".
- No skill cites factory-protocol, worktree-protocol, git-commits, step-decomposition, lessons-codification, bash or rust.

**Hooks:** 23 SKILL.md files mention hooks.
- wave-gate → the `validate-wave-gate-completeness` hook (GATE_CHECK lines).
- deliver-story step-d5 → `validate-per-story-adversary-convergence` WASM hook.
- compact-state → `precompact-flush.wasm` and `postcompact-reanchor.sh`.
- activate / deactivate generate and remove `hooks.json`.
- policy-registry checks that lint hooks exist.
- state-burst cites a retired `verify-state-timestamp-refresh` hook.

## 5. Size outliers, near-duplicates, stubs, stale content

**Largest 15 SKILL.md:** brownfield-ingest 463, wrap 414, toolchain-provisioning 404, sdk-generation 383, repo-initialization 367, state-burst 330, release 317, discovery-engine 315, relocate-artifact 311, demo-recording 297, maintenance-sweep 289, consistency-validation 288, wave-gate 285, factory-health 245, deliver-story 244.

**Smallest 15:** multi-repo-health 22, wave-status 22, phase-5 37, research-cache-ops 38, next-step 39, phase-4 39, phase-6 42, phase-0 45, phase-2 45, validate-workflow 47, phase-3 48, phase-1-spec-crystallization 49, phase-7 50, excalidraw-export 51, deactivate 53.

**Near-duplicates / overlapping pairs:**
- **record-demo (96) vs demo-recording (297):** both record per-AC demo evidence (Playwright/VHS). wave-gate calls `/record-demo`; workflows call demo-recording. They also use different paths (`.factory/demo-evidence/STORY-NNN/` vs `docs/demo-evidence/<STORY-ID>/`).
- **consistency-validation (288) vs validate-consistency (187):** both do cross-document BC/VP/index checks. validate-consistency is the one with `allowed-tools` and fixtures; consistency-validation carries old DF-020/DF-021 numbering.
- **create-brief vs guided-brief-creation (+ brainstorming):** two brief-authoring paths. Workflows use guided-brief-creation and phase-1 uses create-brief.
- **dtu-creation vs dtu-validate:** both describe building and maintaining DTU clones (third-party services vs critical subsystems).
- **convergence-check vs convergence-tracking:** both produce CONVERGED / NOT_CONVERGED. Tracking depends on LiteLLM cost logs and calls itself "the convergence tracker for the Dark Factory".
- **pr-create vs code-delivery vs fix-pr-delivery:**
  - pr-create builds a PR body itself.
  - deliver-story says "Do not compose the PR body yourself" and leaves it to pr-manager (who follows code-delivery).
  - pr-create uses the legacy `STORY-NNN` ID format.
- **wave-scheduling vs decompose-stories/step-d-wave-schedule:** overlapping wave assignment.
- **factory-health vs factory-worktree-health vs check-state-health:** overlapping `.factory` worktree checks. factory-worktree-health carries the dark-factory engine-directory guards.
- **spec-drift vs check-input-drift vs design-drift-detection:** spec-to-code drift, hash drift and design-system drift respectively.
- **formal-verify vs phase-f6-targeted-hardening:** the same four techniques, full scope vs delta.
- **Phase entry skills vs their work skills:** pure indirection.
- **Plugin `wrap` vs the user-level `wrap` skill:** the same name exists outside the plugin.

**Stubs / reference-only:**
- excalidraw-export and jira describe themselves as "Reference-only skill, not directly invokable", but neither sets `disable-model-invocation` or `user-invocable`.
- mode-decision-guide, model-routing, feature-mode-scoping-rules and traceability-extension are reference documents ("no quality gate").
- state-update is `user-invocable: false`.

**Superseded / stale evidence:**
- state-burst has a "History" section describing the retired two-commit protocol (TD-VSDD-053) and a retired hook (ADR-046).
- factory-cycles-bootstrap is a migration from the "flat adversarial-review layout"; adversarial-review warns about legacy `.factory/specs/adversarial-review-pass-*.md` files.
- Legacy `STORY-NNN` IDs appear in pr-create, record-demo, worktree-manage, code-delivery and phase-3, while deliver-story uses canonical `S-N.MM` ("legacy STORY-NNN accepted").
- deliver-story still writes sprint-state and STATE.md itself (Step 9), while wrap insists every STATE.md write goes through state-manager.

**Referenced but missing in the plugin:**
- `export-png.sh` (excalidraw-export)
- `generate-pdf.sh` and `templates/1898-report.css` (generate-pdf)
- `tdd-enforcement.sh` (phase-f4 step-04)
- `config/verification-toolchains.yaml` (toolchain-provisioning; `config/` holds only `artifact-path-registry.yaml`)
- `openclaw.json` (agent-file-review, phase-1d)
- `.claude/agents/codebase-analyzer.md` (brownfield-ingest; the agent actually lives at `agents/codebase-analyzer.md`)

**Project-specific leakage:**
- brownfield-ingest and disposition-pass hardcode `/Users/jmagady/Dev/corverax/.reference/...` and talk about the "Corverax sandbox" and "Corverax vision".
- deliver-story cites "Prism commits aa706543…"; validate-consistency cites Prism.
- generate-pdf is branded "1898 & Co."
- Rust/cargo is hardcoded in deliver-story, wave-gate, formal-verify, perf-check, setup-env, toolchain-provisioning and state-burst.

## 6. Legacy vocabulary and non-Claude models

- **OpenClaw:** agent-file-review (`openclaw.json` tool profiles), phase-1d ("Adversary model configured in `openclaw.json`"), session-review ("OpenClaw tool config").
- **Dark Factory / dark-factory:**
  - agent-file-review, market-intelligence-assessment, convergence-tracking, toolchain-provisioning, sdk-generation, session-review ("Story in dark-factory repo").
  - factory-worktree-health and repo-initialization carry ~20 `*"dark-factory"*` FATAL guards, and repo-initialization says LLM keys "live in the dark-factory repo's .env".
  - semport-analyze and brownfield-ingest use `../dark-factory` as their example input.
- **LiteLLM / multi-provider routing:**
  - model-routing tiers: judgment = Claude Opus 4.6, implementation = Sonnet 4.6, validation = Haiku 4.5, adversary = GPT-5.4 ("NEVER Claude"), review = Gemini 3.1 Pro, fallbacks = DeepSeek-V3 / Qwen3-235B / Codestral via OpenRouter. It also gives per-token prices and "LiteLLM proxy at localhost:4000".
  - repo-initialization: "Verify all 3 model families are reachable via LiteLLM proxy". convergence-tracking reads "LiteLLM cost data".
- **GPT-5.4 named:** phase-4 gate ("Evaluator used different model family (GPT-5.4, not Claude)"), phase-f2 step-06 ("adversary agent (GPT-5.4, fresh context)").
- **Gemini named:** phase-5 ("Optional Gemini secondary pass"), phase-f4 ("pr-reviewer (Gemini…)"), phase-f5 plus step-g (`gemini-review.md`, "Gemini's 1M context").
- **"Different model family" requirement** (assumes non-Claude providers): phase-1d plus steps, phase-4, phase-5, phase-f5 plus steps, holdout-eval. code-delivery and fix-pr-delivery describe pr-reviewer as the "4th model family".
- **Codex / Gemini CLI:** visual-companion (visual-guide.md and `start-server.sh` auto-detect `CODEX_CI`).
- **Lobster runtime assumptions:**
  - run-phase, next-step and validate-workflow interpret `.lobster` step fields themselves.
  - session-review says "Lobster context.exclude config".
  - design-drift-detection says "Sweep 10 in maintenance.lobster".
- **`sessions_spawn`:** no hits.
- **Claude model names / tiers in bodies:**
  - model-routing (Opus 4.6, Sonnet 4.6, Haiku 4.5); session-review example ("used Opus where Sonnet would suffice"); agent-file-review lists "claude-haiku" among forbidden strings.
  - deliver-story and all its step files use abstract tiers (Fast / Standard / Capable), and step-d5 assigns the adversary "Capable".
- **Effort settings:** none for Claude. market-intelligence-assessment and discovery-engine mention Perplexity `reasoning_effort: high|medium|low|minimal`.

## 7. `model:` and effort in frontmatter

No skill frontmatter contains `model:`, `effort:` or any reasoning or thinking key. The only model-adjacent frontmatter is the four `context: fork` + `agent:` pairs and the five `disable-model-invocation` flags. All model selection lives in body text (model-routing, the deliver-story tier table, the phase-gate "different model family" prose).
