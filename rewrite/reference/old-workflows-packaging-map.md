<!-- Map of the pre-rewrite .lobster workflows, packaging, tooling, templates, CI and release. Captured 2026-11-23 by a read-only survey of branch develop @ 150bc36c. Historical reference for the 1.0 rewrite; not maintained. -->

# vsdd-factory: map of the plugin outside agents, skills and hooks

Three things a revamp will run into early:

1. **The `.lobster` files have no runtime.** They are YAML that the orchestrator agent and a few skills read as data. All parallelism, loops, gates, context walls and `model_tier` in them are honour-system only.
2. **The cross-vendor model routing in the docs is not wired up in Claude Code.** The docs say the adversary runs on GPT-5.4 and Gemini does secondary review via LiteLLM and `openclaw.json`. Neither file exists in the repo, and the adversary agent's frontmatter says `model: opus`.
3. **`/Users/jmagady/dev2/vsdd-factory/.mcp.json` contains live API keys** for Perplexity, Tavily and Context7. It is gitignored (`.gitignore:70`) and not tracked, so it is local-only, but don't copy it into a repackaged plugin.

Repo state: branch `develop`, HEAD `150bc36c`. `.factory/` is not mounted in this checkout (it's gitignored and `git worktree list` shows only the main tree). The `origin/factory-artifacts` branch exists.

---

## 1. Plugin packaging

**Manifests found**
- The only `plugin.json` in the repo is `/Users/jmagady/dev2/vsdd-factory/plugins/vsdd-factory/.claude-plugin/plugin.json`:
  ```json
  { "name": "vsdd-factory",
    "description": "Verified Spec-Driven Development (VSDD) dark factory for software — full SDLC pipeline: brownfield ingest, spec crystallization, story decomposition, TDD delivery, adversarial review, holdout evaluation, formal verification, and release gating.",
    "version": "1.0.0-rc.25",
    "author": { "name": "drbothen" },
    "homepage": "https://github.com/drbothen/vsdd-factory",
    "repository": "https://github.com/drbothen/vsdd-factory",
    "license": "MIT",
    "keywords": ["vsdd","spec-driven","tdd","factory","sdlc","agents","pipeline"] }
  ```
  It declares no `hooks`, `mcpServers`, `commands`, `outputStyles` or `userConfig` fields, so Claude Code uses its default directory discovery.
- There is no `marketplace.json` anywhere in the repo. It was removed in rc.7 (see `scripts/bump-version.sh` header).
- The marketplace is a separate repo, `drbothen/claude-mp`. Its `.claude-plugin/marketplace.json` points here with:
  `"source": {"source":"git-subdir","url":"https://github.com/drbothen/vsdd-factory.git","path":"plugins/vsdd-factory","ref":"main"}`
  This is documented in `/Users/jmagady/dev2/vsdd-factory/docs/guide/plugin-marketplace-architecture.md` (§3–5). That doc also explains why a marketplace and plugin nested in the same repo silently failed to populate the install cache from rc.2 to rc.6.

**Install, update and cache path**
- Install: `/plugin marketplace add drbothen/claude-mp`, then `/plugin install vsdd-factory@claude-mp`.
- Update: `/plugin marketplace update claude-mp`, then `/plugin update vsdd-factory@claude-mp`.
- Local development: `claude --plugin-dir ./plugins/vsdd-factory`.
- Cache: `~/.claude/plugins/cache/claude-mp/vsdd-factory/<version>/`. CLAUDE.md and RELEASING.md stress that edits on `develop` don't reach operators until a release ships.

**Repo-root `.claude/` and `.lazyclaude/`**
- `.claude/settings.json` (tracked): `{"enabledPlugins": {"vsdd-factory@claude-mp": true}}`.
- `.claude/scheduled_tasks.lock` (tracked): a session lock JSON.
- `.claude/pr-reviews/55.json` and `65.json` (tracked): stored PR review verdicts and findings (`pr_number`, `reviews[]`, findings with severity, category, agent, file, line, hash).
- `.lazyclaude/worktrees/<hash>`: committed as a gitlink (mode 160000) with no `.gitmodules`. The directory is empty on disk.

**`.mcp.json` (repo root, untracked)**: defines the MCP servers `perplexity` (stdio, npx), `tavily` (http), `playwright` (stdio) and `context7` (http). This is where the live keys are.

**Hooks wiring**
- `plugins/vsdd-factory/hooks/hooks.json` is gitignored (`.gitignore:54`). The `activate` skill copies one of the per-platform files into place: `hooks.json.{darwin-arm64,darwin-x64,linux-arm64,linux-x64,windows-x64}`.
- Those files are generated from `hooks.json.template` by `scripts/generate-hooks-json.sh`.
- The template routes 10 events (PreToolUse, PostToolUse, PermissionRequest, Stop, SubagentStop, SessionStart with `once`, SessionEnd with `once`, WorktreeCreate, WorktreeRemove, PostToolUseFailure) to one command: `${CLAUDE_PLUGIN_ROOT}/hooks/dispatcher/bin/{{PLATFORM}}/factory-dispatcher{{EXE_SUFFIX}}`, timeout 10000.
- Also at the plugin root: `hooks-registry.toml` (63 KB, the dispatcher's config) and `resolvers-registry.toml` (WASM context resolvers, e.g. `wave_context`).

**Stale counts in the README**: it says version rc.22 (actual rc.25), 16 workflows (17), 5 bin helpers (21), 99 templates (136), 9 rules (10). It lists a `commands/` directory with 108 commands that doesn't exist, and a CI workflow `plugin-validation.yml` that doesn't exist.

---

## 2. `plugins/vsdd-factory/workflows/` (17 files)

### Files

| File | Lines | `workflow.name` / version | Named steps | What it defines |
|---|---|---|---|---|
| `greenfield.lobster` | 1417 | greenfield-vsdd 2.1.0 | 116 | Reference path: repo init, factory worktree gate, planning sub-workflow, phases 1–7, release, steady-state handoff, session review |
| `feature.lobster` | 1489 | feature-vsdd 3.0.0 | 121 | Delta phases F1–F7; routing variants trivial, bug-fix and critical-bug |
| `brownfield.lobster` | 400 | brownfield-vsdd 3.0.0 | 26 | Phase 0 ingest, then the greenfield sub-workflow |
| `code-delivery.lobster` | 445 | per-story-delivery 2.0.0 | 32 | Reusable per-story flow (worktree → stubs → Red Gate → implement → demo → PR → reviews → merge), with typed `inputs:` |
| `discovery.lobster` | 435 | autonomous-discovery 2.0.0 | 29 | Opportunity research → briefs → route to planning |
| `maintenance.lobster` | 418 | maintenance-sweep 2.0.0 | 36 | 11 scheduled sweep types; fix PRs go through code-delivery |
| `multi-repo.lobster` | 731 | multi-repo-vsdd 3.0.0 | 62 | `project.yaml` manifest, repo-level waves (Kahn's algorithm), per-repo sub-workflows with `cwd`, cross-repo gate |
| `planning.lobster` | 298 | adaptive-planning 2.0.0 | 24 | Front-end: detect artifacts, market intel, brief, route |
| `phases/phase-0-codebase-ingestion.lobster` | 146 | v3.0.0 | 15 | Broad-then-converge ingest |
| `phases/phase-1-spec-crystallization.lobster` | 161 | v2.0.0 | 16 | Brief → domain spec → PRD/BCs → architecture → VPs → UX |
| `phases/phase-2-story-decomposition.lobster` | 171 | v1.0.0 | 16 | Epics, stories, dependency graph, wave schedule, holdout scenarios |
| `phases/phase-3-tdd-implementation.lobster` | 158 | v2.0.0 | 18 | Per-story TDD dispatch |
| `phases/phase-4-holdout-evaluation.lobster` | 38 | v1.0.0 | 3 | Scenario rotation (80%) → `holdout-eval` → gate |
| `phases/phase-5-adversarial-refinement.lobster` | 54 | v1.0.0 | 4 | "GPT-5.4 primary + Gemini 3.1 Pro secondary" loop |
| `phases/phase-6-formal-hardening.lobster` | 91 | v2.0.0 | 9 | Kani, fuzz, mutation, security scan |
| `phases/phase-7-convergence.lobster` | 175 | v2.0.0 | 19 | 7-dimension check, demo, visual review, human approval |
| `phases/per-story-delivery.md` | 228 | (Markdown with frontmatter) | n/a | Per-story playbook: stub discipline, Red Gate, RED_RATIO density check, facade mode |

`phases/per-story-delivery.md` is not identical to `agents/orchestrator/per-story-delivery.md` (266 lines). Both exist.

### Step schema (keys counted across all `.lobster` files)

- **Workflow header:** `workflow.{name, description, version, defaults{on_failure, max_retries, timeout}, cost_monitoring{enabled, metadata{mode}}, steps[]}`, plus `inputs`, `config` and `schedule` in some files.
- **Step keys:**
  - Core: `name` (562), `type` (546), `depends_on` (507), `task` (286), `agent` (283), `condition` (166), `skill` (130, as a path like `"skills/<x>/SKILL.md"`), `timeout` (88).
  - Context walls: `context.{include, exclude}` (49 steps, written as `# ▓ WALL:` comments plus globs).
  - Gates: `gate.{criteria[], fail_action}` (40).
  - Human approval: `approval.{prompt, artifacts[], timeout}` (37).
  - Sub-workflows: `sub_workflow` (25, e.g. `"planning.lobster"`).
  - Model: `model_tier` (24; values are only `adversary` and `review`).
  - Loops: `loop.{max_iterations, exit_condition, for_each, steps[]}` (24).
  - Other: `optional`, `cwd`, `iterator` / `collection` / `for_each` (in parallel-foreach steps), `on_failure`, `max_retries`.
- **`type` values:** agent 280, skill 130, gate 38, human-approval 37, sub-workflow 25, loop 24, parallel-foreach 8, multi-repo 4, parallel 2, compound 2, ui 1. There is no `id` field; `name` is the identifier.
- **Conditions are pseudo-expressions:** `"!file_exists('CLAUDE.md')"`, `"architect.verdict == 'request-changes'"`, `"feature_type in ['ui','full-stack']"`. Exit conditions look like `"adversary.verdict == 'CONVERGENCE_REACHED'"`, and `for_each: wave in wave-schedule.md`.

### How they are executed

**There is no Lobster runtime.** The Lobster runtime came from the OpenClaw/NemoClaw "dark-factory" this was ported from (`docs/FACTORY.md:13-17`, `docs/not-portable.md`).

`bin/lobster-parse` (bash, 51 lines) is just `yq eval -o json` piped to `jq "$EXPR"`. The files are consumed as data in these places:
- `agents/orchestrator/orchestrator.md:8-37`: "Read them as data — do not treat them as prose", followed by `lobster-parse` examples.
- `skills/run-phase/SKILL.md`: the LLM resolves the file, extracts `{name,type,agent,depends_on,task}`, topologically sorts it, and then:
  - `type: agent` → Task/Agent tool
  - `type: skill` → Skill tool
  - `type: command` → Bash
  - honours `on_failure` and `max_retries`, appends progress to `.factory/STATE.md`.
- `skills/next-step/SKILL.md`: proposes the next step from STATE.md plus the workflow.
- `skills/validate-workflow/SKILL.md`: static schema check.
- `rules/step-decomposition.md`: a three-layer model (top-level lobster → phase entry-point skill → phase lobster → `skills/<x>/steps/step-*.md`). Step `name`s must match step file IDs.

The only mechanical checks are parse tests: `tests/bin.bats` ("all 15 workflow files parse", which is now stale) and `.github/workflows/ci.yml:94-97`, which runs `lobster-parse … '.workflow.name'` on each file.

### Representative excerpt (`greenfield.lobster:634-673`)

```yaml
    - name: phase-3-per-story-delivery
      type: loop
      depends_on: [dx-engineer-preflight-gate, pre-phase-4-dtu-gate, pre-phase-4-cicd-gate]
      timeout: "12h"
      loop:
        for_each: wave in wave-schedule.md
        steps:
          # --- Create worktrees for this wave ---
          - name: create-worktrees
            type: agent
            agent: devops-engineer
            task: >
              Create git worktrees for all stories in this wave:
              git worktree add .worktrees/STORY-NNN -b feature/STORY-NNN develop
              One worktree per story. .worktrees/ is gitignored.

          # --- Per-story: stub -> test -> implement -> demo -> push -> PR -> review -> merge ---
          - name: per-story-delivery
            type: parallel
            depends_on: [create-worktrees]
            for_each: story_group in wave.groups
            steps:
              - name: stub-architect
                type: agent
                agent: test-writer
                task: >
                  Create compilable stubs for STORY-NNN in .worktrees/STORY-NNN/.
                  cargo check must pass. Do NOT write tests or implementation.

              - name: test-writer-step
                type: agent
                agent: test-writer
                depends_on: [stub-architect]
                task: >
                  Write failing tests for STORY-NNN in .worktrees/STORY-NNN/.
                  Tests must compile but all fail (Red Gate). Do NOT implement.

              - name: implementer-step
                type: agent
                agent: implementer
                depends_on: [test-writer-step]
                task: >
                  Implement STORY-NNN via TDD in .worktrees/STORY-NNN/.
```

The header style (lines 37–89) uses the same shape: `type: skill` + `skill:` path, a `type: gate` with `gate.criteria`, `condition` / `optional`, and `type: agent` with `agent: state-manager`.

---

## 3. bin/, tools/, scripts/, ci/, benches/, tests/

### `plugins/vsdd-factory/bin/` (21 files, all bash)

| File | Purpose |
|---|---|
| `check-stale-verdict.sh` | Compares the PR's head SHA against the `covered_sha` in a READY verdict before merging (BC-5.42.001, ADR-030) |
| `compute-input-hash` | MD5 of an artifact's frontmatter `inputs`; supports `--update`, `--check`, `--resolve`, `--scan` |
| `emit-event` | Always-exit-0 JSONL event emitter to `.factory/logs/events-YYYY-MM-DD.jsonl` |
| `enforce-merge-strategy.sh` | The only allowed `gh pr merge` wrapper; forces `--merge` for `release/v*` |
| `factory-cas-push.sh` | Fetch, then `--force-with-lease` CAS push of `factory-artifacts` |
| `factory-dashboard` | Markdown dashboard built from STATE.md, wave-state.yaml and events |
| `factory-lock-acquire-precheck.sh` | Decision tree for `/factory-lock` acquire (BC-6.23.001) |
| `factory-lock-status.sh` | Three-state lock display |
| `factory-lock-write.sh` | Acquire, renew or clear the `factory_lock` frontmatter in STATE.md (TTL 2700 s) |
| `factory-unlock-decide.sh` | Unlock decision helper (`--force` support) |
| `factory-obs` | `docker compose` lifecycle for the observability stack; multi-factory registry |
| `factory-query` | Canned queries over the event log (`top`, `recent`, …) |
| `factory-replay` | Per-`session_id` playback of hook activity |
| `factory-report` | Daily, weekly or range markdown reports |
| `factory-sla` | Subagent durations from `agent.start`/`agent.stop` event pairs |
| `lobster-parse` | yq + jq workflow reader |
| `multi-repo-scan` | Scans `.worktrees/` and `.reference/` manifests and emits a JSON dependency graph |
| `research-cache` | SHA-keyed cache at `.factory/research-cache/<sha>.json` |
| `resolve-worktree-identity.sh` | Resolves and asserts the 4-field worktree identity for per-story adversary dispatch |
| `validate-template-compliance.sh` | Checks VPs with more than one `bcs` entry have a valid `source_bc` |
| `wave-state` | Queries `sprint-state.yaml` for wave readiness |

### `plugins/vsdd-factory/tools/observability/` (15 files)

- `README.md`: the stack is opt-in.
- `docker-compose.yml`: Loki 3.6.10, busybox, otel-collector-contrib 0.149.0, Prometheus 3.5.2, Grafana 13.0.1, grafana-image-renderer.
- Config: `otel-collector-config.yaml`, `loki-config.yaml`, `prometheus-config.yaml`.
- 7 Grafana dashboard JSON files: `factory-prs`, `factory-roi`, `factory-subagents`, `factory-today`, `factory-overview`, `claude-cost`, `claude-overview`.
- 3 provisioning YAML files (two datasources, one dashboard provider).

### Repo-root `scripts/`

| File | Language | Purpose |
|---|---|---|
| `bump-version.sh` | bash | Inserts a CHANGELOG stub. No longer touches `plugin.json` |
| `check-changelog-monotonicity.sh` | bash | CHANGELOG dates must be strictly descending |
| `check-platforms-drift.py` | python | `ci/platforms.yaml` must match the inline `ci.yml` matrix |
| `check-shakedown-window.sh` | bash | Beta-shakedown window gate (BC-9.01.006) |
| `generate-hooks-json.sh` | bash | Renders the template into 5 platform variants |
| `generate-registry-from-hooks-json.sh` | bash | One-time v0.79 hooks.json → `hooks-registry.toml` converter |
| `legacy/hooks-json-pre-templating.json` | JSON | Old per-script hooks.json (bash hooks wired directly) |

### Other root directories

- `ci/platforms.yaml` (YAML): canonical list of the 5 target platforms (platform, os, Rust target, `use_cross`, `run_tests`).
- `benches/legacy-adapter-latency.sh` (bash): latency of bash hooks called directly vs. through dispatcher + legacy-bash-adapter.
- `tests/` (35 files, bats):
  - `bats/`: registry lint, async-event schema, envelope sync, VP079 mutation proof.
  - `integration/E-8-hook-plugins/`: 9 WASM-hook suites.
  - `integration/hooks/`: block-helper, canonical-format, session-learning, plus fixtures.
  - `perf/`: E-8 artifacts gate and bash baseline, plus JSON fixtures.

---

## 4. templates/, rules/, config/, fixtures/, docs/

### `templates/` (136 files)

The top level has 106 files, mostly `*-template.md`. By category:
- **Spec hierarchy:** product-brief, L2-domain-spec (+ index/section), prd, 4 prd-supplements, behavioral-contract, L4-verification-property, architecture (+ index/section), adr, verification-architecture and coverage matrix, ux-spec (+ index/screen/flow), module-criticality.
- **Stories and waves:** epic, epic-index, story, story-index, wave-schedule, `wave-state-template.yaml`, cycle-manifest.
- **Reviews:** adversarial-review / finding / index, code-review, security-review, review-findings, consistency reports, convergence report and trajectory, holdout scenario / index / evaluation (×3).
- **Delivery:** pr-description, red-gate-log, demo templates (`.tape`, `.spec.ts`, CI YAML), demo-evidence-report.
- **Operations:** state, burst-log, session-checkpoints, lessons, tech-debt-register.
- **Config YAML:** policies, merge-config, autonomy-config, discovery-config, project-manifest, reference-manifest, ui-traceability.
- **DTU, gene-transfusion, brownfield:** recovered-architecture, conventions, project-context.
- **Misc:** `project-justfile-template`, `verify-sha-currency.sh`.

Subdirectories:
- `adversary-prompt-templates/` (3): phase-1d spec review, phase-2 story review, phase-5 code review.
- `ui-quality/` (4).
- `design-system/` (23): `constraints.yaml`, 3 pattern YAMLs, 7 token JSONs, `component-registry.yaml`, 11 component contract YAMLs.

Consumers: referenced as `${CLAUDE_PLUGIN_ROOT}/templates/…` by 71 skills, 27 agents and 4 bash hooks (most-cited: pr-description, story, wave-schedule, product-brief). `hooks/validate-template-compliance.sh` checks output against them. No Rust crate references them.

### `rules/` (10 files)

| File | Subject |
|---|---|
| `_index.md` | `@`-import list of 7 rules (omits `lessons-codification` and `step-decomposition`) |
| `bash.md` | Shell error-handling standards |
| `factory-protocol.md` | `.factory/` worktree, layout, commit protocol |
| `git-commits.md` | Conventional Commits |
| `lessons-codification.md` | Adversary process catch → codification follow-up |
| `rust.md` | Rust rules (still says "for Corverax") |
| `spec-format.md` | L1–L4 hierarchy and formats |
| `step-decomposition.md` | Phase numbering, three-layer workflow model |
| `story-completeness.md` | Story self-containment checklist |
| `worktree-protocol.md` | Branch and worktree strategy |

Loading: there is no plugin mechanism for these. Claude Code plugins have no `rules/` component, and nothing imports `_index.md`. Skills point at them by text, e.g. `create-story/SKILL.md:52` says `.claude/rules/story-completeness.md`, a path that doesn't exist in the plugin. `scaffold-claude-md` says the plugin "provides those via its agents and rules system".

### `config/artifact-path-registry.yaml` (477 lines)

The canonical `.factory/` path patterns, each with an `enforcement_level` (ADR-016). Consumed by the WASM hook `crates/hook-plugins/validate-artifact-path` and by `crates/last-amended-migrate`. Writes to unregistered `.factory/` paths are blocked.

### `fixtures/smoke-project/`

A trivial Rust crate (`Cargo.toml` plus `src/lib.rs` with `add()`). Plugin tests live in `plugins/vsdd-factory/tests/` (107 entries).

### `plugins/vsdd-factory/docs/`

- `VSDD.md`: methodology. Phases 1, 2, 3, 5, 6 and 7 have headings; there is no Phase 4 section.
- `FACTORY.md` (1092 lines): a "constitution" still written for OpenClaw (`openclaw.json`, LiteLLM, `DARK_FACTORY_HOME`, `nemoclaw` commands).
- `CONVERGENCE.md`: 7 dimensions.
- `AGENT-SOUL.md`: 13 principles.
- `bash-portability.md`: lint guard.
- `not-portable.md`: 4 OpenClaw extensions that weren't ported (cost-tracker, attention-heatmap, tiered-context, full sidecar-learning).

---

## 5. The pipeline as documented

**Phases** (`rules/step-decomposition.md`): 0 Codebase Ingestion · 1 Spec Crystallization (sub-steps 1a–1d) · 2 Story Decomposition · 3 TDD Implementation · 4 Holdout Evaluation · 5 Adversarial Refinement · 6 Formal Hardening · 7 Convergence (+ Release). Feature mode uses F1–F7: Delta Analysis, Spec Evolution, Incremental Stories, Delta Implementation, Scoped Adversarial, Targeted Hardening, Delta Convergence.

Inconsistencies: `greenfield.lobster` and `FACTORY.md` say "six phases". The README says "five convergence dimensions" while CONVERGENCE.md has 7. FACTORY.md still uses fractional phases (3.5, 4b), which step-decomposition forbids.

**Modes** (`docs/guide/workflow-modes.md`, 8): Greenfield, Brownfield (Phase 0 then greenfield), Feature (steady state), Maintenance (scheduled, 11 sweeps), Discovery, Planning (embedded front-end, not standalone), Multi-Repo, Code Delivery (sub-workflow). Routing: no code → greenfield; code without `.factory/` → brownfield; existing `.factory/` → feature. `docs/guide/pipeline-paths.md` lists 14 paths.

**`.factory/` layout** (`rules/factory-protocol.md`, `FACTORY.md:37-91`):
- `STATE.md`
- `specs/`: product-brief, `domain-spec/L2-INDEX.md`, `prd.md`, `prd-supplements/`, `behavioral-contracts/BC-INDEX.md` + `ss-NN/BC-S.SS.NNN.md`, `verification-properties/VP-INDEX.md`, `architecture/ARCH-INDEX.md` + `decisions/ADR-*`, `research/`, dtu-assessment, gene-transfusion-assessment
- `stories/`: STORY-INDEX, epics, dependency-graph, `sprint-state.yaml`
- `cycles/vX.Y.Z-<mode>/`: manifest, adversarial-reviews, convergence-report, wave-schedule, burst-log, decision-log, lessons, release-notes
- `holdout-scenarios/`: HS-INDEX, `wave-scenarios/`, `evaluations/`
- `dtu-clones/`, `semport/`, `code-delivery/`, `demo-evidence/`, `logs/`
- `policies.yaml`, `merge-config.yaml`, `autonomy-config.yaml`, `wave-state.yaml`

Lifecycle classes: living, accumulating, cycle-scoped.

**Orphan branch and worktree mechanism**
- `.factory/` is a git worktree on the orphan branch `factory-artifacts`, created by the `repo-initialization` and `factory-worktree-health` skills as the first blocking steps of greenfield.
- Only state-manager commits there, with messages `factory(<phase>): …`, one commit per burst (TD-VSDD-053).
- Pushes go through `bin/factory-cas-push.sh`.
- CI mounts it with `git worktree add .factory origin/factory-artifacts` (`ci.yml:21-25`).
- Story work uses `.worktrees/S-N.MM/` on `feature/S-N.MM-*` branches from `develop`.
- Multi-repo uses `.factory-project/` on `factory-project-artifacts`.

**Waves**
- `wave-scheduling` skill: topological sort; stories with no dependencies go in Wave 1.
- A wave runs stories in parallel, each in its own worktree.
- After all of a wave's stories merge, the Wave Integration Gate runs: full suite on develop, an adversary loop on the wave diff, security review if the wave has CRIT/HIGH modules, holdout regression, wave demo, and a fix loop of up to 10 rounds.
- The Wave Gate has 6 checks under the Iron Law "no wave advance without all six".

**Convergence**
- CONVERGENCE.md uses Novelty = N/(N+D), converged below 0.15 for 2 or more consecutive passes. Median severity must be strictly decreasing for 3 or more passes and below 2.0.
- Other dimensions: mutation kill rate by module criticality, Kani/fuzz/security, holdout, visual, documentation.
- 3-CLEAN rule: `adversarial-review/SKILL.md` requires a minimum of 3 consecutive clean passes (0 CRIT, 0 HIGH), maximum 10. The `hooks/convergence-tracker.sh` hook enforces this when `CONVERGENCE_REACHED` is written, along with novelty ≤ 0.15. BC-5.39.001 says any finding resets the streak. CLAUDE.md:230 notes it is "currently structurally impossible", so the engine runs in "asymptotic acceptance" (D-386 Option C).
- A zero-findings first pass is treated as suspicious and re-run.

**Holdout**
- Hidden HS-NNN scenarios (and WHS-W[N]-NNN per wave) are created in Phase 2; builders never see them.
- `holdout-eval` runs with `context: fork` and `agent: holdout-evaluator`.
- Each run rotates to a random 80% of scenarios.
- Gate: mean satisfaction ≥ 0.85, every must-pass ≥ 0.6, standard deviation < 0.15.

**DTU (Digital Twin Universe)**
- The architect writes `dtu-assessment.md` in Phase 1 (`DTU_REQUIRED` true/false).
- `dtu-creation` builds clones of external services; `pre-phase-4-dtu-gate` checks they exist; `dtu-validate` and `dtu-validator` check fidelity.
- The glossary defines DTU differently, as a clone of a reference implementation.

**Spec hierarchy**: L1 Product Brief → L2 Domain Spec (CAP, DI, DEC, ASM, R, FM) → L3 PRD + Behavioral Contracts BC-S.SS.NNN (plus NFR, error taxonomy, subsystems SS-NN) → architecture (ARCH-INDEX, ADRs) → L4 Verification Properties VP-NNN (immutable once green) → Epics and Stories (AC traces to BC) → tests → code → proofs. 12 governance policies live in `.factory/policies.yaml`; the IDs are catalogued in `docs/guide/id-reference.md` ("30 identifier formats").

---

## 6. Model selection and cross-vendor review

**Mechanisms that actually exist in Claude Code**
- Agent frontmatter `model:`: 7 agents use `opus` (adversary, codebase-analyzer, holdout-evaluator, pr-reviewer, formal-verifier, spec-reviewer, research-agent) and 27 use `sonnet`. Nothing uses haiku, and no agent sets `effort:`.
- `.lobster` `model_tier: adversary|review` on 24 steps. Nothing interprets it.
- No skill sets `model:` or `effort:`.

**Cross-vendor intent, documented only**
- `docs/FACTORY.md:400-440`: LiteLLM proxy at `localhost:4000`. Tiers: judgment = Opus 4.6, implementation = Sonnet 4.6, validation = Haiku 4.5, adversary = GPT-5.4 ("NEVER Claude"), review = Gemini 3.1 Pro, plus Codestral, DeepSeek-V3 and Qwen3 fallbacks. Points at `config/litellm-config.yaml` and `openclaw.json`, neither of which is in the repo.
- `skills/model-routing/SKILL.md`: the same table as a reference skill.
- `docs/VSDD.md:203,279`: adversary should be GPT-5.4 or DeepSeek.
- `docs/AGENT-SOUL.md` §9 "Cognitive Diversity Is a Feature".
- `agents/code-reviewer.md:22`: "you are a DIFFERENT model family than the Builder (Claude)". Same idea in the adversary's description.
- Workflows and phase-4/5 gates require "different model family (GPT-5.4, not Claude)". The secondary "Gemini" review steps and `gemini-review.md` filenames are recognised by the `convergence-tracker` and `validate-novelty-assessment` hooks.
- `templates/merge-config-template.yaml:23`: `pr_review_model: "litellm/pr-review/primary"  # 4th model family`.
- `skills/repo-initialization/SKILL.md:341`: "Verify all 3 model families are reachable via LiteLLM proxy".
- `docs/not-portable.md`: per-model cost tracking and Opus→Sonnet→Haiku downgrade can't be ported to plugin hooks.

**Other model mentions**
- "Codex" appears only in Rust comments recording past adversarial passes done out-of-band ("CROSS-VENDOR (OpenAI Codex)" in `crates/factory-dispatcher/src/shard_manager.rs` and `tests/bc_1_18_008_*`), plus `CODEX_CI` detection in the visual-companion script.
- "effort" matches are mostly "best-effort" comments. The real ones are Perplexity `reasoning_effort: "high"` in `security-reviewer.md` and the discovery skills.
- "thinking" matches are prose only.

---

## 7. `.github/workflows/` and the release procedure

| Workflow | Trigger | Purpose |
|---|---|---|
| `ci.yml` | push/PR to main and develop | `validate` (mount factory-artifacts, shell syntax, bats, manifest JSON, hooks.json variants match template, `plugin.json` version valid, parse every `.lobster`); `cargo-host` (fmt, clippy, test, wasm32-wasip1 plugin builds, perf baseline); policy-15 attestation gate + non-vacuity controls; `platforms-drift`; 5-platform `build-dispatcher`; bats full suite (linux, macOS legs); `deny-advisories` |
| `kani.yml` | push/PR | Kani crash-recovery proofs (OBL-1) |
| `release-branch-guardrail.yml` | PR events | Fails if a `release/*` head branch targets anything other than `main` |
| `release.yml` | tag `v*` | `validate` → `build-binaries` (5 platforms) → `commit-binaries` → `release` (GitHub prerelease) → `bump-marketplace` → `sync-develop` |
| `semgrep.yml` | PR, push, weekly cron | SAST |

`commit-binaries` is a bot commit on `main`. It:
- stages dispatcher binaries under `hooks/dispatcher/bin/<platform>/` and `.wasm` files under `hook-plugins/`
- regenerates the `hooks.json.*` variants
- writes `plugin.json` version from the tag
- force-moves the tag to that commit.

`bump-marketplace` opens a PR on `drbothen/claude-mp` and needs the `CLAUDE_MP_PAT` secret; without it the job skips itself.

**Procedure** (`/Users/jmagady/dev2/vsdd-factory/RELEASING.md`, which calls itself the single source of truth):
1. Branch `release/v<semver>` from `develop`.
2. Run `scripts/bump-version.sh` and move the `[Unreleased]` entries into a `## <semver>` CHANGELOG section.
3. Open a PR to `main` and wait for all 10 checks.
4. Merge with `--merge` (never `--squash`).
5. Create an annotated tag `v<semver>` at `origin/main`'s new tip.
6. Watch `release.yml`, then merge the claude-mp PR.
7. Verify `schema_version` in the cache copy of `hooks-registry.toml`.

Versions are rc-style prereleases (`1.0.0-rc.N`). Current: `plugin.json` 1.0.0-rc.25, and the latest CHANGELOG heading is `1.0.0-rc.25` (2026-09-04) below `## [Unreleased]`.

CONTRIBUTING.md: feature PRs target `develop` and are squash-merged. It also covers the Semgrep policy and the Rust 1.95.0 toolchain.

---

## 8. Root `docs/`

- `guide/` (30 files, 476 KB): user and dev guides. Getting started; workflow-modes; pipeline-paths; pipeline-overview; per-phase guides for 0, 1, 2, 3, 5, 6, 7 (no Phase 4 guide); agents, commands, hooks, templates, policy and ID references; glossary; configuration; troubleshooting; observability and observability-sinks; visual-companion; plugin-marketplace-architecture; v1.0 kit docs (v1.0-index, authoring-hooks, porting-bash-hook-to-wasm, migrating-from-0.79, semver-commitment).
- Top-level: `dispatch-package-authoring.md`, `E-8-tier1-native-audit.md`, `oq-6-regression-gate-security-audit.md`.
- `lessons-learned/wave-gate-bookkeeping.md`.
- `demo-evidence/` (110 MB, 1313 files, 87 story dirs `S-3.x` through `S-25.x`): per-story demo-recorder output committed on feature branches. Mostly `.md`, `.txt`, VHS `.tape`, `.gif`, `.webm`.

---

## 9. Claude Code native features: what's used and what isn't

**Used**
- Skill frontmatter `context: fork` + `agent:` in 4 skills: `adversarial-review` (adversary), `holdout-eval` (holdout-evaluator), `research` (research-agent), `spec-drift` (Explore).
- `disable-model-invocation` (e.g. `activate`, `deactivate`, `state-burst`, `adversarial-review`), `user-invocable` (`state-update`), `allowed-tools` (about 30 skills), `argument-hint`.
- Plugin hooks through `hooks.json`, including WorktreeCreate/Remove, PermissionRequest, PostToolUseFailure and PreCompact/PostCompact handling (CLAUDE.md:261-269).
- `activate` writes the project's `.claude/settings.local.json` so `orchestrator` becomes the main-thread agent.
- `check-state-health` checks `env.CLAUDE_AUTOCOMPACT_PCT_OVERRIDE` in settings.json.
- `.gitignore:15-18` mentions `Agent(isolation: "worktree")` leftovers under `.claude/worktrees/`.
- `docs/guide/plugin-marketplace-architecture.md` §7 documents the SKILL.md `model`, `effort`, `context`, `agent` and `paths` fields.

**Not present anywhere** (excluding target/ and demo-evidence): the Workflow tool or Workflow scripts, `.claude/workflows`, `workflow-authoring`, output styles, agent `effort` / `isolation` / `memory` / `permissionMode` / `maxTurns` / `skills` frontmatter, plugin-level `mcpServers`, `userConfig`, `CLAUDE_PLUGIN_DATA`, a `commands/` directory, plugin `settings.json`, and agent teams.
