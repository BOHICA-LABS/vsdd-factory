# vsdd-factory — Project Operating Instructions

> Read this file first. Every other doc in this repository operates under the principle stated below.

## Project Identity

**vsdd-factory** is the open-source factory engine implementing Verification-Specification-Driven Development (VSDD) — a multi-agent orchestration system that turns product briefs into production-grade code via specialist agents, adversarial review, and asymptotic convergence cycles. Distributed as a Claude Code plugin (`vsdd-factory:*` agents and skills).

**Self-referential:** vsdd-factory IS the project being onboarded. Engine and product are the same repository. The "don't use dark-factory paths as cwd" rule in upstream skills does NOT apply here. `.factory/` writes target this repo intentionally. The marketplace-tarball at `~/.claude/plugins/cache/claude-mp/vsdd-factory/<version>/` is the operator-level consumed copy; the `plugins/vsdd-factory/` source in this repo is the development copy. Source edits must be released for hooks to pick them up at the operator level — develop-branch changes don't affect the cached plugin.

Mode: brownfield-onboarding (engine-discipline F5 asymptotic convergence cycle; engine itself is the product being refined).

## Current Pipeline State

Read `.factory/STATE.md` for live state. As of last commit on this branch:
- Cycle: `v1.0-feature-engine-discipline-pass-1` (F5 cycle-level adversarial-review asymptotic convergence per D-386 Option C).
- Mode: brownfield-onboarding with concurrent feature-engine-discipline cycle.
- Convergence trajectory: asymptotic floor [7,9] HIGH findings per pass; META-LEVEL ply ascending monotonically (L1..L24 confirmed). PR #124 OPEN DRAFT CI-GREEN; merge gated on streak progression OR explicit human stop signal.

## Build / Test / Lint

```bash
# Bats integration suite — the canonical end-to-end test for the dispatcher + hook chain
cd plugins/vsdd-factory/tests && ./run-all.sh

# Cargo workspace — Rust crates including dispatcher binary + hook-sdk + context resolvers
cargo test --workspace --all-targets

# Format + lint (CI enforces these exact flags on every PR)
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings

# Combined pre-push gate (mirrors ci.yml)
cargo fmt --check --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace --all-targets
```

CI runs the same suite on every PR to `develop` or `main` via `.github/workflows/ci.yml`. The cargo-cache reuse story across PR + release.yml runs is tracked as TD #70 (Option C — release pipeline build-time optimization).

## Architectural Authority — Source of Truth

When two artifacts disagree, the LATER, MORE-SPECIFIC artifact wins:

1. **`.factory/STATE.md`** — live pipeline state (current phase, decisions log D-NNN, session resume checkpoint, frontmatter `current_step`). Recent decision rows (D-NNN, latest = D-449) supersede earlier-recorded but conflicting narrative. The Session Resume Checkpoint (11 sections) is the canonical post-clear/post-compact resume source.
2. **`.factory/cycles/v1.0-feature-engine-discipline-pass-1/decision-log.md`** — full decision-log SoT (D-379..D-449). STATE.md Decisions Log table is a summary; decision-log.md Appendix prose is authoritative for sub-clause expansion.
3. **`.factory/cycles/v1.0-feature-engine-discipline-pass-1/lessons.md`** — L-EDP1-NNN cumulative lessons (L-EDP1-001..061). META-LEVEL ply taxonomy (L1..L24) lives here with 1-sentence definitions per D-447(b)+D-449(c).
4. **`.factory/cycles/v1.0-feature-engine-discipline-pass-1/burst-log.md`** — per-burst structural record. D-444(c) requires 8 mandatory blocks per entry (Parent-commit, Adversary verdict, Files touched, Codifications, Dim-2/5/6/7 Attestations, Closes, Factory-artifacts commits). D-448(a) requires source-attestation parity with the adversary review file.
5. **`.factory/cycles/v1.0-feature-engine-discipline-pass-1/INDEX.md`** — adversarial reviews table + Convergence Status row. Authoritative for pass-count + fix-burst-count + D-NNN range + 4-index version cite.
6. **`.factory/specs/architecture/ARCH-INDEX.md`** — subsystem registry; canonical source for subsystem names (POLICY 6 in `.factory/policies.yaml`).
7. **`.factory/specs/behavioral-contracts/BC-INDEX.md`** — BC catalog. BC H1 is authoritative BC title (POLICY 7). bcs frontmatter array changes propagate atomically to body BC table + ACs + Token Budget (POLICY 8).
8. **`.factory/specs/verification-properties/VP-INDEX.md`** — authoritative VP catalog. Changes propagate same-burst to verification-architecture.md + verification-coverage-matrix.md (POLICY 9).
9. **`.factory/stories/STORY-INDEX.md`** — story catalog. Per D-448(b)+D-449(d)(iii), frontmatter changelog migration to structured `changelog:` array is deferred to S-15.03 PRIORITY-A; current inlined `last_amended`-string form is documentary-historical-deferred per D-414(c).
10. **`.factory/policies.yaml`** — declarative governance policy registry (12 baseline policies). Adversarial-review skill auto-loads this file into the adversary dispatch rubric. Lint hooks read it to determine which validators to run.
11. **`RELEASING.md`** — canonical release procedure. The `/vsdd-factory:release` skill defers to it. Release branches MUST be named `release/v<full-semver>` targeting `main` (enforced by `.github/workflows/release-branch-guardrail.yml`, TD #69). Release PRs MUST be merged with `--merge` (not `--squash`) to preserve develop's commits as ancestors of main.
12. **For code-vs-spec conflicts**: the SPEC wins (Standing Rule for VSDD). Code is brought into alignment via fix-burst or follow-up story, not the other way around. Only the human can authorize spec amendment to match code.

If two artifacts are at the same precedence level and disagree, surface to the orchestrator. The orchestrator routes to the artifact's owner-specialist (e.g., BC vs BC → product-owner; ADR vs ADR → architect) for adjudication.

## Pipeline Authority

The orchestrator (`vsdd-factory:orchestrator` agent) coordinates all phases. Specialist agents do the writing. **The orchestrator does NOT write files itself** — it delegates via the `Agent` tool with `subagent_type` set to the specialist (see Agent Routing Table in the Companion Principle section below). The single permitted exception is direct human-mandated edits to this CLAUDE.md or other project-root meta-docs.

vsdd-factory's engine-discipline cycle operates differently from standard greenfield/brownfield pipelines: it is in continuous F5 cycle-level adversarial-review asymptotic convergence per D-386 Option C. Per-cycle structure:

- **Dispatch-side advance** (state-manager; D-417(b) strict — only `phase:` + `current_step:` modified)
- **Cycle-level adversary** (fresh-context per Iron Law; reads ONLY prior-pass Part A)
- **Fix burst Commits A/B/C/D/E** (single commit per role per D-381+TD-VSDD-053 single-commit-per-burst)
  - **Commit A:** adversary review persist + INDEX.md row + Convergence Status + retroactive remediations + pass-N burst-log h2 heading
  - **Commit B:** decision-log D-NNN codification block + canonical 6-column rows + L-EDP1-NNN lesson entry (with Closes block per D-448(b))
  - **Commit C:** S-15.03 cumulative-scope advance + retroactive sweeps
  - **Commit D:** 4-index version bumps (BC/VP/STORY/ARCH) + Convergence Status advance + Active Branches atomic advance per D-445(c)+D-446(d)
  - **Commit E:** STATE.md final advance (banner cumulative-cite + dual-margin per D-446(c) + Phase Progress row + Concurrent Cycles tail LENGTH=4 per D-433(e)+D-439(c) + Last Updated + Current Phase + Session Resume Checkpoint refresh + frontmatter advance per D-443(a)+D-444(a)+D-449(a) literal shell execution gates) + pass-N burst-log 8-block completion
  - **SHA-patch follow-up:** Active Branches → actual Commit E HEAD per D-447(c)+D-449(e)

Per-story Phase 3 sub-workflow (when stories are in flight): stubs → failing tests → TDD green → LOCAL adversary 3-CLEAN → demo-recorder per-AC → push → pr-manager 9-step PR cycle → squash-merge → state-manager post-merge burst. BC-5.39.001 3-CLEAN protocol applies to every cascade.

## CANONICAL PRINCIPLE — Production-Grade Default

This principle binds every AI agent operating on this project. It overrides any default behavior in agent prompts, skills, or templates that conflicts with it.

### Statement

**Default behavior is enterprise/production-grade correctness. Speed lives in feature *ordering*, not feature *completeness*.**

### Six rules

1. **No MVP-driven deferrals.** Phrases like "for now," "good enough," "we can fix later," "minimum viable," and "ship fast and iterate" are RATIONALIZATIONS, not engineering decisions. Treat them as defect-pattern smells. If a thing is worth doing in v1, it is worth doing correctly in v1.

2. **Feature order is the only acceptable speed lever.** It is acceptable to defer an entire feature (e.g., a future story or wave) to a later cycle. It is NOT acceptable to ship the current story partially or with shortcuts that need later cleanup. Each shipped feature must be production-grade, enterprise-ready, on the cycle it ships.

3. **Tech debt register (`.factory/tech-debt-register.md`) is for HUMAN-DIRECTED deferrals ONLY.** AI agents must NOT add entries to it as a default catchment for issues found during review. If an agent discovers a defect, the default action is to FIX it in-scope. Adding to the register requires ALL of:
   - Explicit human direction to defer, AND
   - A concrete future dependency that makes the deferral necessary (e.g., "this depends on S-15.03 PRIORITY-A automation"), AND
   - Attachment to the specific future story or wave where it will be resolved (so it cannot get lost).

4. **AI-built defects are the AI's responsibility to fix.** Every artifact in `.factory/` and most code in `crates/` and `plugins/vsdd-factory/` was written by AI (with human approval). When an AI agent finds an issue in another AI agent's output, the default is to fix it in the current scope — even if that means expanding scope. Surfacing the issue as a question, an "advisory," a "TODO for architect," or a "pending architect review" is the WRONG default. The correct default is to fix.

5. **`Suggest` is acceptable. `Default to cheap path` is not.** Agents may propose cheaper alternatives to the human, but the agent's DEFAULT action must be the correct path. "I noticed this would be faster if we skipped X — would you like to?" is fine. Skipping X without surfacing the option is not.

6. **"Pending architect review" / "TODO for architect" / "Placeholder for architect" in spec artifacts is forbidden when the question is answerable in current scope.** If the question requires architect adjudication only because the answer needs cross-component reasoning that hasn't happened yet, that's legitimate. If the question is mechanical (path migration, version pin selection, conventional clippy lint configuration), the AI handling the spec must answer it now.

### What this means in practice

| Anti-pattern | Production-grade replacement |
|--------------|------------------------------|
| "MVP: ship without test coverage on edge case X" | Write the edge case test. Cover it now. |
| "For now we'll hardcode this value; refactor later" | Read the value from config now. Write the config schema. |
| "We can add error handling in v2" | Add error handling now. Define the error taxonomy in scope. |
| "Architect TODO: confirm patch-version pinning policy" | Pick the production-grade default and write the rationale inline. |
| "Pending architect review: should we support 6 endpoints?" | Read the canonical contract, decide based on existing parity argument, document the decision. |
| "Phase 5 deferred: add this to tech-debt-register" | First ask: did the human direct this deferral? If no, fix it now. |
| "Good enough for v1" | "Production-grade for v1." If you can't say production-grade, you're not done. |
| Implementer claims "MVP scope" / "test-path-only" / "deferred to follow-up" | Adversary independently verifies the claim under fresh-context analysis (Standing Rule 3 §1). Implementer self-disclosure of risk severity is NOT authoritative. |
| Silent `Vec::new()` return where partial-failure data should propagate | Thread proper plumbing through; surface-and-defer-via-error is a SOUL.md #4 violation (Standing Rule 3 §2). |
| Doc comment claiming "this requires capability X" with no capability check | Either implement the gate or remove the docs (Standing Rule 3 §3). |
| Adding plumbing to a constructor that didn't have it, to close a finding correctly | DO IT. "Wiring not redesign" means don't *replace* existing implementations; it does NOT mean don't *add* proper plumbing where it was missing (Standing Rule 3 §4). |
| File a P4 TD for cosmetic cleanup of 2 byte-identical types (~45 min total) | Fix the 2 cosmetic cleanups in-scope. P4 TDs that could have been a single inline edit are a defer-pattern smell. |
| Pseudocode gate attestation (`extract <foo> | diff`) in burst-log Dim-2 | Execute literal shell with captured stdout per D-449(a). META-LEVEL-24 reveals narrative-attested gates cannot detect their own scope-degradation. |

### Self-Audit Checklist (every agent, before declaring work done)

Run this checklist as the last act of every task. If any answer is "yes" or "I'm not sure," stop and remediate before declaring done.

- [ ] Did I rationalize any decision with "MVP," "for now," "good enough," or "we can fix later"?
- [ ] Did I add a new tech-debt-register entry without **all three** of: explicit human direction, concrete future dependency, and a specific future story/wave anchor?
- [ ] Did I leave any "pending architect review," "TODO for architect," or "Placeholder for architect" in a spec artifact for a question I could have answered in scope?
- [ ] Did I find a bug or gap in another AI's output and surface it as a question/advisory instead of fixing it in scope?
- [ ] Did I default to the cheapest mechanism instead of the correct mechanism?
- [ ] If I added an ADVISORY-severity finding to a report, did I evaluate whether it should be a BLOCKER under the production-grade lens? (Most "advisories" become blockers.)
- [ ] Did I paper-fix a finding by renaming, doc-commenting, or asserting-only when the real fix is structural? (TD-VSDD-059 paper-fix detection.)
- [ ] Did I sibling-sweep all callsites when I changed a function signature, constant, or canonical identifier? (TD-VSDD-060 sibling-site sweep.)
- [ ] If I codified a mechanical gate (D-444(a)-class diff gate, D-446(a)-class block-presence gate, D-448(a)-class source-attestation gate), did I INVOKE it via literal shell with captured stdout per D-449(a) — not pseudocode narrative? (META-LEVEL-24 closure.)

### Boundaries — what the principle does NOT mean

- **It does not mean "do everything before shipping anything."** Phasing cycles (engine-discipline pass-N → pass-N+1) is correct. Within a cycle, every shipped feature must be production-grade.
- **It does not mean "no asks of the human."** Genuine human decisions — risk acceptance, business priorities, scope vs deadline tradeoffs, versioning policy — should be surfaced. The principle forbids deferring WORK that the AI can do; it does not forbid surfacing DECISIONS that only the human can make.
- **It does not mean "infinite scope expansion."** If you find an issue, fix it. If the fix requires expanding into a new domain that requires new specs or new architecture decisions, surface it cleanly and request scope expansion. The principle requires fixing, not infinite recursion.
- **It does not override security or correctness.** If a "production-grade fix" requires a security review, run the security review.

### Companion Principle — Correct Agent Routing

"Fix in scope" works ONLY when paired with correct agent routing. Otherwise it degrades into "every agent does everything," which destroys specialization and produces worse work than the defer-pattern it replaces.

#### Rules

1. **Agents own their domain.** A consistency-validator does NOT silently rewrite spec content. An implementer does NOT silently rewrite the spec. Each specialist agent has a defined scope (see Agent Routing Table below); work outside that scope is routed to the correct specialist via the orchestrator.
2. **The orchestrator owns routing.** When a specialist agent discovers a defect outside its own domain, it surfaces the finding to the orchestrator with the proposed routing. The orchestrator then dispatches the correct specialist. This is NOT a defer-pattern — it is correct-agent-pattern. The fix still happens in scope of the same work cycle.
3. **Surface vs defer — the critical distinction:**
   - **Surface (production-grade):** Agent A finds issue → routes to orchestrator → orchestrator dispatches specialist B → specialist B fixes in scope. **No human round-trip required for the routing.**
   - **Defer (forbidden):** Agent A finds issue → adds to tech-debt-register / advisory / "TODO for X" → original work declared done → issue persists. **Requires human to discover and re-prioritize.**
4. **When in doubt about routing, ask the orchestrator** — not the human. The orchestrator has the routing table loaded; let it route.
5. **The orchestrator NEVER does specialist work itself.** It coordinates, dispatches, and validates gates. If the orchestrator is tempted to write a file directly (other than this CLAUDE.md per direct human mandate), that is a routing failure — find the correct specialist and dispatch.

#### Agent Routing Table

Use this table to determine which specialist handles which kind of work. Authoritative reference; supersedes any conflicting routing in upstream skills.

| If the work is... | Route to agent ID |
|-------------------|-------------------|
| Product brief, PRD, behavioral contracts (BCs), holdout scenarios | `vsdd-factory:product-owner` |
| Market analysis, L2 domain spec, ubiquitous language | `vsdd-factory:business-analyst` |
| Architecture, ADRs, DTU assessment, gene transfusion, dependency manifest | `vsdd-factory:architect` |
| UX spec, design system, wireframes, interaction design | `vsdd-factory:ux-designer` |
| Story decomposition, dependency graph, wave schedule | `vsdd-factory:story-writer` |
| Cross-document consistency (IDs, anchors, counts, naming) | `vsdd-factory:consistency-validator` |
| Adversarial fresh-context review (specs or implementation) | `vsdd-factory:adversary` |
| Constructive spec/story review (different-model cognitive diversity) | `vsdd-factory:spec-reviewer` |
| PR diff code review (different-model cognitive diversity) | `vsdd-factory:code-reviewer` |
| Deep codebase scanning, semantic analysis, brownfield ingest | `vsdd-factory:codebase-analyzer` |
| Brownfield extraction validation (catch hallucinated dependencies) | `vsdd-factory:validate-extraction` |
| TDD test stubs and failing tests | `vsdd-factory:test-writer` |
| TDD implementation (one failing test → minimum code → micro-commit) | `vsdd-factory:implementer` |
| E2E browser tests (Playwright/Cypress) | `vsdd-factory:e2e-tester` |
| Demo recordings (VHS terminal or Playwright browser) | `vsdd-factory:demo-recorder` |
| PR lifecycle (create, review dispatch, finding triage, merge) | `vsdd-factory:pr-manager` |
| Final fresh-eyes PR diff review before merge | `vsdd-factory:pr-reviewer` |
| Formal proofs (Kani), fuzzing, mutation testing, security scan | `vsdd-factory:formal-verifier` |
| Security review / triage (CWE/CVE, OWASP) | `vsdd-factory:security-reviewer` |
| Holdout scenario evaluation against implementation (strict info asymmetry) | `vsdd-factory:holdout-evaluator` |
| DTU clone validation against real third-party services | `vsdd-factory:dtu-validator` |
| Repo setup, worktrees, CI/CD, release, Cargo workspace init | `vsdd-factory:devops-engineer` |
| Toolchain preflight, env setup, dependency installation | `vsdd-factory:dx-engineer` |
| `.factory/STATE.md` updates, `.factory/` commits, cycle bookkeeping | `vsdd-factory:state-manager` |
| Spec governance, versioning, traceability audit | `vsdd-factory:spec-steward` |
| Documentation generation from code/specs (current behavior only) | `vsdd-factory:technical-writer` |
| External research (Perplexity, Context7, Tavily MCP access) | `vsdd-factory:research-agent` |
| GitHub CLI operations on behalf of agents without shell access | `vsdd-factory:github-ops` |
| Performance benchmarks, regression detection | `vsdd-factory:performance-engineer` |
| Data schemas, migrations, pure-core / effectful-I/O boundary | `vsdd-factory:data-engineer` |
| WCAG AA/AAA accessibility audit | `vsdd-factory:accessibility-auditor` |
| Visual regression, mockup fidelity comparison | `vsdd-factory:visual-reviewer` |
| Post-pipeline analysis, lessons capture, improvement proposals | `vsdd-factory:session-reviewer` |

#### Routing examples (from vsdd-factory's engine-discipline history)

- **Cycle-level adversarial finding** during F5 convergence loop: correct routing is `vsdd-factory:state-manager` (owner of `.factory/` artifacts including STATE.md, decision-log, lessons, burst-log, 4 indexes). The orchestrator dispatches per the per-burst Commit A/B/C/D/E sequence.
- **META-LEVEL ply codification** discovered by adversary: correct routing is `vsdd-factory:state-manager` for the fix burst; the codification IS the fix. The adversary identifies the ply class; state-manager codifies D-NNN + L-EDP1-NNN at Commits B+E.
- **Cross-document consistency defect found by consistency-validator** during a phase gate: correct routing is `product-owner` (owner of BC/PRD content) OR `architect` (owner of ADR content), NOT consistency-validator-fixes-it. The orchestrator dispatches.
- **TDD red-gate violation found by test-writer** where a Red Gate test does not align with the BC: route to `product-owner` (if the BC is the problem) or to the human (if the spec is genuinely contradictory). DO NOT have the test-writer modify the BC silently.
- **Security finding found by security-reviewer**: triage classification is security-reviewer's job. The FIX is implementer's job (with security-reviewer re-running to confirm). Use the `fix-pr-delivery` skill.
- **Out-of-scope finding (legitimate scope-boundary defer)**: still route to orchestrator. Orchestrator records the deferral with explicit future-story attachment per Canonical Principle Rule 3. The deferral target must be a real story ID, not "later" or "next cycle."

#### When the routing is unclear

If a defect doesn't obviously map to a specialist:

1. **Ask the orchestrator first.** The orchestrator has the routing table loaded; let it route.
2. **If the orchestrator is uncertain, the orchestrator asks the human.** This is the legitimate use of human time — routing-table extensions, not domain-fixes-by-wrong-agent.
3. **Default fallback for unmapped work: research → architect.** Most truly novel work that doesn't fit a specialist needs external research first (`vsdd-factory:research-agent`), then architectural decision (`vsdd-factory:architect`).

#### Anti-patterns this principle blocks

- ❌ Adversary rewrites failing tests "to make them pass" (wrong: route to test-writer or implementer).
- ❌ State-manager writes spec content like BC bodies or ADR rationale (wrong: route to product-owner or architect; state-manager handles index rows, frontmatter syncs, decision logs, and cross-document version bumps).
- ❌ Consistency-validator silently edits brief frontmatter (wrong: route to product-owner).
- ❌ Implementer adds a new BC to fix a TDD red-gate (wrong: route to product-owner; implementer cannot author specs).
- ❌ Orchestrator writes the artifact itself when a specialist's output is unsatisfactory (wrong: re-dispatch the specialist with better instructions, or escalate to human).
- ❌ Any agent edits `.factory/STATE.md` directly (wrong: state-manager owns STATE.md).
- ❌ Filing a P4 "opportunistic cleanup" TD when the fix is ~45 minutes of in-scope work (wrong: fix in-scope per Canonical Principle Rule 3 + Rule 4).
- ❌ Narrative pseudocode attestation in burst-log Dim-2 when the codification specifies a mechanical gate (wrong: invoke literal shell with captured stdout per D-449(a); META-LEVEL-24 self-application).

### Operational Discipline TDs (vsdd-factory-specific layering)

These project-specific operational rules layer onto the canonical principle. Recorded in `.factory/STATE.md` Decisions Log and enforced by the factory-dispatcher hook chain:

- **TD-VSDD-053 — Single-commit-per-burst.** Each logical burst → ONE commit in `.factory/`. Multi-commit chains (HEAD and HEAD^ both containing "backfill" / "Stage 1" / "Stage 2") trigger `MULTI_COMMIT_CHAIN_NOT_ALLOWED`. Recovery procedure documented in "Factory Hook Diagnostics" below.
- **TD-VSDD-059 — Paper-fix detection.** State-manager and adversary must verify every claimed closure has a load-bearing test or assertion, not just a doc-comment or rename. Implementer self-disclosure of risk severity is NOT authoritative — adversary independently verifies.
- **TD-VSDD-060 — Sibling-site sweep on value changes.** When changing a function signature, constant, or canonical identifier, grep for ALL callsites in the same crate (and adjacent crates if `pub`) before committing.
- **TD-VSDD-091 — Anti-volatile-pin.** Narrative spec content must cite function names + behavioral anchors, NOT `file.rs:NNN` line numbers (which decay on subsequent diffs). Justified citations (Red Gate test tables, AC source-of-truth tables, pass-report changelogs) excepted.
- **BC-5.39.001 — 3-CLEAN convergence protocol.** Adversarial cascades require three consecutive clean passes for convergence; any finding resets the streak to 0/3. Applies to both LOCAL and PR-LEVEL cascades. Currently structurally impossible under prose-only codification per L-EDP1-007/051/061 — the F5 cycle-level loop operates per D-386 Option C asymptotic acceptance.
- **TD-FACTORY-HOOK-BYPASS-001 P0** — Use Edit/Write tools ONLY for `.factory/` mutations. NEVER use Python/sed/echo bypass. Enforced by POL-3.
- **POL-14 — Auto-promotion at merge.** When a story's PR merges, BCs in `behavioral_contracts` frontmatter auto-promote `draft → active`. State-manager runs this transition.
- **D-417(b) — Dispatch-side advance strict.** State-manager dispatch-side advance commits modify ONLY `phase:` + `current_step:` in STATE.md frontmatter. No other fields.
- **D-419(b)+D-420(d)+D-421(a) — Parent-commit-SHA convention.** Commit E's current_step cites Commit D's SHA (not Commit E's own SHA). Downstream dispatches cite the prior pass's Commit D SHA.
- **D-433(e)+D-439(c) — Trajectory tail LENGTH=4.** STATE.md trajectory tail narrative shows exactly 4 arrow-separated axis-count values (`→9→9→9→9`).
- **D-441(a)+D-442(a)+D-443(a)+D-444(a)+D-449(a) — Verbatim-strict chain on current_step.** No meta-commentary; no clause-reordering; no justification-suffix injection; all prescribed clauses present; diff gate INVOKED via literal shell at Commit E (D-449(a) closes META-LEVEL-24).
- **D-446(a) — Own-burst-log 8-block gate.** Every fix-burst Commit E verifies its OWN burst-log entry contains all 8 D-444(c) blocks before push.
- **D-448(a) — Source-attestation gate.** Burst-log Adversary verdict paragraph MUST faithfully describe adv-cycle-pass-N.md Part A finding set. Literal shell diff at Commit E (per D-449(a)).
- **D-449(a) — Literal-shell-execution-evidence.** Mechanical gates (D-444(a)/D-446(a)/D-448(a)) require literal shell invocation + captured stdout in burst-log Dim-2. Pseudocode FORBIDDEN. META-LEVEL-24 acknowledgment.

## Conventions (Code-Level)

vsdd-factory-specific coding patterns enforced by CI and/or adversarial review. These are non-negotiable under the production-grade default — violations are bugs, not style preferences.

### Highlights

- **Single-workspace MSRV.** Toolchain pinned via `rust-toolchain.toml` (if present at root) or per-crate `rust-version`. No per-crate MSRV divergence. All workspace crates build on the single pinned channel.

- **Dispatcher binary discipline.** The factory-dispatcher binary (`plugins/vsdd-factory/hooks/dispatcher/bin/<platform>/factory-dispatcher`) is this project's own consumed dispatcher. Source lives in `crates/`. Release pipeline cross-compiles for darwin-arm64 / darwin-x86_64 / linux-x86_64 / linux-musl / windows-x86_64. Edits to source must be RELEASED (cut a new rc tag + push) for the operator-level marketplace-tarball cache to pick them up. Develop-branch edits do not affect the cached plugin.

- **Hook registry semantics.** `plugins/vsdd-factory/hooks-registry.toml` declares every hook plugin (52 plugins as of v1.0.0-rc.16). Each entry specifies plugin name, WASM path, trigger events (PreToolUse / PostToolUse), tier (sync vs async), and on-error behavior (block / advisory). The registry is config; the dispatcher is the runtime. POL-3 enforces no-bypass.

- **WASM plugin fuel budgets.** Hook plugins run in a WASM sandbox with a finite fuel budget. Large factory artifacts (e.g., `.factory/cycles/v1.0-feature-engine-discipline-pass-1/lessons.md` >3000 lines) can exhaust fuel during validation, surfacing as `block_intent=true exit_code=2` PostToolUse blocks. Per D-442(e), `lessons.md` size budget is ≤3500 soft / ≤4000 hard; remediation (compact, split, or fuel-budget increase) deferred to S-15.03 PRIORITY-A.

- **No `println!` in production Rust code.** Use `tracing::*!` with structured fields only. `println!` is restricted to examples and CLI formatting helpers.

- **Error taxonomy.** Use named error variants. No `unwrap()` or `expect()` in critical code paths.

- **Bash hook scripts.** Bash hooks (e.g., `convergence-tracker.sh`) use `set -euo pipefail`. A crashed bash hook with `on_error=block` produces `exit_code=2` PostToolUse signals. Hook scripts in `plugins/vsdd-factory/hooks/*.sh` must be robust to empty / pre-existing-defect input — silent crashes propagate as false-positive blocks that, while non-destructive to file writes (PostToolUse is post-write), pollute session telemetry.

- **Context compaction — three distinct mechanisms (S-18.07 / ADR-026 §Decision 7):**

  | Mechanism | What it is | Trigger |
  |-----------|-----------|---------|
  | `/compact-state` skill | Human/agent-initiated skill (`plugins/vsdd-factory/skills/compact-state/SKILL.md`). Extracts historical content from STATE.md into cycle files (burst logs, adversary passes, session checkpoints, lessons) and slims STATE.md to <200 lines. It does NOT invoke the Claude Code `/compact` command — it reorganizes the STATE.md file, not the conversation context. | Explicit operator or agent invocation only. |
  | `PreCompact` hook event | Automatic Claude Code harness event fired before context compaction. Triggers the `precompact-flush` PreCompact WASM plugin (`precompact-flush.wasm`), native WASM per ADR-028 §Decision 2 / S-18.04a, which persists wave-boundary state to `factory-artifacts` before context is lost. | Fired by the harness on automatic compaction (not by `/compact-state`). |
  | `PostCompact` hook event | Automatic Claude Code harness event fired after compaction completes. Triggers `postcompact-reanchor.sh`, which emits a `[PostCompact Re-anchor]` block to stdout so the LLM session can re-ground itself. | Fired by the harness on automatic compaction (not by `/compact-state`). |

  **Invoking `/compact-state` does NOT fire the `PreCompact` or `PostCompact` hook chains.** These are independent mechanisms. After any session clear or context reset (whether from automatic compaction or a manual clear), run `/rehydrate-wave` as the first step before any pipeline work.

### Forbidden patterns

| Pattern | Reason |
|---------|--------|
| Direct edit of `.factory/STATE.md` by any agent other than state-manager | State-manager owns STATE.md (Routing Table) |
| Multi-commit chain in `.factory/` (HEAD + HEAD^ both contain "backfill" / "Stage N") | TD-VSDD-053 single-commit-per-burst; triggers `MULTI_COMMIT_CHAIN_NOT_ALLOWED` |
| `--no-verify` git flag | Bypasses hook chain; TD-FACTORY-HOOK-BYPASS-001 P0 |
| `Co-Authored-By: Claude` or any AI attribution in commit messages | Explicit human directive for vsdd-factory |
| Force-push to `main` | Release branches merge with `--merge`; main is append-only |
| Pseudocode gate attestation (`extract <foo>`, narrative `→ output empty`) in burst-log Dim-2 | D-449(a) requires literal shell + captured stdout; META-LEVEL-24 violation |
| Stale parent-commit-SHA citation (citing the prior-prior pass's Commit-D instead of latest) | D-419(b)+D-420(d)+D-421(a) parent-commit-SHA convention |
| O-PXX-NNN observation IDs in 4-index changelog Refs | D-449(d)(i) scope: Refs covers findings + PG only, not observations |

## Git Workflow

### Branch model

- **Default branch:** `main` (release branch; receives one merge per release + one bot commit for the binary bundle)
- **Active integration:** `develop` (every feature/fix PR targets here)
- **Release branches:** `release/v<full-semver>` (e.g., `release/v1.0.0-rc.17`); targets `main`, merged with `--merge` (TD #69 guardrail enforces both invariants)
- **Feature branches:** `feature/<story-id>` (e.g., `feature/S-12.08`)
- **Maintenance branches:** `maintenance/<scope>`
- **Factory artifacts branch:** `factory-artifacts` (orphan branch mounted at `.factory/` via worktree). Holds spec docs, cycle logs, STATE.md. Pushed to origin by state-manager during fix bursts.

### Commit conventions

- **Conventional Commits format preferred** (`feat:`, `fix:`, `chore:`, `state:`, `cycle:`) but not enforced by hook for non-`.factory/` commits.
- **Factory hook chain** (`.factory/` commits): single-commit-per-burst per TD-VSDD-053; `MULTI_COMMIT_CHAIN_NOT_ALLOWED` detector blocks two consecutive commits with "backfill" / "Stage 1" / "Stage 2" in their subjects. See "Factory Hook Diagnostics" section below for the full recovery procedure.

### Non-negotiable git rules

- **NEVER skip hooks** (`--no-verify`, `--no-gpg-sign`). If a hook fails, investigate and fix the underlying issue. Bypassing is a TD-FACTORY-HOOK-BYPASS-001 P0 violation.
- **NEVER add AI attribution to commits** — no `Co-Authored-By: Claude`, no robot emojis. The user has explicitly directed this for vsdd-factory.
- **NEVER force-push to `main`.** Force-push to `develop` requires explicit human approval. Force-push to feature/maintenance branches is acceptable when the work is local-only (no collaborators); `--force-with-lease` preferred over raw `--force`.
- **NEVER use destructive operations as a first-line response.** `git reset --hard`, `git clean -f`, `git checkout --` should be the last option after exhausting safer alternatives (`git stash`, `git reset --soft`, worktree-based isolation).

### Operational tips

- **Heredoc workaround:** large commit-message heredocs are sometimes blocked by hook payload limits. When `git commit -m "$(cat <<'EOF' ... EOF)"` fails, write the message to `/tmp/<file>` and use `git commit -F /tmp/<file>`.
- **Soft reset for recovery, never `--hard`.** Per the multi-commit-chain recovery procedure: `git -C .factory reset --soft HEAD~N` preserves the working tree state; re-author as a single combined commit.
- **`git stash` for in-progress work** when context-switching between worktrees — preserves uncommitted changes without losing them to a reset.

## Releases

**Read [`RELEASING.md`](./RELEASING.md) before cutting any release.** It is the canonical procedure. The release skill (`/vsdd-factory:release`) defers to it.

Critical invariants:
- Release branches MUST be named `release/v<full-semver>` and MUST target `main` (enforced by `.github/workflows/release-branch-guardrail.yml`, TD #69).
- Release PRs MUST be merged with `--merge` (not `--squash`) to preserve develop's commits as ancestors of main.
- Tag the release at main's new tip after the PR merges.
- After every release, the local plugin cache at `~/.claude/plugins/cache/claude-mp/vsdd-factory/<version>/` contains the marketplace-tarball version of the dispatcher and registry. Operator-level cache picks up source edits only after release; develop-branch edits do not affect the cached plugin.

## Hooks (this project's own dispatcher)

This project ships the dispatcher binary it consumes. The full hook chain documentation lives in `plugins/vsdd-factory/hooks-registry.toml` (declarative) and the dispatcher source in `crates/` (runtime). 52 plugins as of v1.0.0-rc.16.

## Factory Hook Diagnostics

When `Agent` tool dispatches or Edit/Write operations fail with errors like:

```
PreToolUse:Agent hook error: [...factory-dispatcher]: factory-dispatcher trace=<UUID> event=PreToolUse tool=Agent host_abi=1 matched_tiers=N plugins_run=N total_ms=N block_intent=true exit_code=2
```

— the factory-dispatcher hook chain blocked the dispatch. The error message itself carries NO human-readable reason — only the trace UUID. To diagnose, follow this procedure.

### Step 1 — Locate the dispatcher log

Internal logs live at:

```
.factory/logs/dispatcher-internal-YYYY-MM-DD.jsonl
```

(One file per day, JSONL format, one event per line.)

### Step 2 — Find the block reason

**As of TD #71 (v1.0.0-rc.17+), the block reason is surfaced directly in the dispatcher stderr summary line.** Look at the stderr output you already have from Step 1's context — blocking dispatches now include `blocking_plugins=<name(s)>` and `block_reason="<text>"` inline:

```
factory-dispatcher trace=<UUID> event=PreToolUse tool=Agent host_abi=1 sync_plugins=N async_plugins=N
  plugins_run=N total_ms=N block_intent=true exit_code=2 blocking_plugins=plugin-a,plugin-b block_reason="FAIL: MULTI_COMMIT_CHAIN_NOT_ALLOWED — HEAD and HEAD^ both contain 'backfill'..."
```

The `blocking_plugins` field names the guard(s) that fired; `block_reason` is the plugin's block message with newlines escaped as `\n`. **For most blocking dispatches, no log grep is required.**

The internal log grep is now needed only for advisory-level `plugin.log` records (non-blocking telemetry), or for debugging crash/timeout fail-closed blocks where the plugin couldn't emit a reason. In those cases, search the day's log for the trace UUID:

```bash
grep '<TRACE-UUID>' .factory/logs/dispatcher-internal-$(date +%Y-%m-%d).jsonl
```

Look for `plugin.log` entries with `level: warn` for advisory context, or `plugin.crashed` / `plugin.timeout` records for fail-closed diagnostics. The `plugin_name` field on each record (e.g., `validate-wave-gate-prerequisite`, `validate-pr-merge-prerequisites`, `regression-gate`, `convergence-tracker`) tells you which guard fired.

### Step 3 — Common blockers and recovery procedures

| Blocker | Detection | Recovery |
|---------|-----------|----------|
| **Multi-commit chain (TD-VSDD-053)** | HEAD and HEAD^ both have `backfill` / `Stage 1` / `Stage 2` in their commit messages | `git -C .factory reset --soft HEAD~N` (preserves working tree); re-author as one combined commit; force-push with `--force-with-lease` (requires explicit user approval) |
| **SHA drift** | STATE.md cites a develop/main SHA that doesn't match `git rev-parse origin/develop` | Update narrative via state-manager dispatch; STATE.md `develop_head` and Active Branches row SHAs must match actuals |
| **In-progress narrative** | STATE.md decision log has an open phase without closure | Add closure row via state-manager; bump version |
| **factory-artifacts dirty** | `git -C .factory status --porcelain` is non-empty | Commit/discard pending changes via state-manager |
| **convergence-tracker.sh exit 1** | Bash hook crash on burst-log/lessons.md edits; `set -euo pipefail` failure | Known false-positive on `.factory/cycles/*pass-[0-9]*.md` path pattern; PostToolUse so writes succeed; advisory only. Root fix scoped to S-15.03 PRIORITY-A hook path-pattern narrowing. |
| **WASM fuel exhaustion on lessons.md** | Large lessons.md (>3000 lines) triggers fuel exhaustion in PostToolUse validators; block_reason reads `fail-closed: FUEL_EXHAUSTED: fuel cap of {N} units exhausted; raise fuel_cap or reduce payload size` (distinct from `fail-closed: plugin timed out`, the epoch/wall-clock arm, which is unchanged) | Per D-442(e), size budget ≤3500 soft / ≤4000 hard; PostToolUse cannot revert writes; advisory. `plugin.timeout` JSONL record includes `fuel_cap` field (alongside `fuel_consumed`) — check it for the configured budget. `DEFAULT_FUEL_CAP` raised to 20,000,000 on develop (ADR-042) but **not yet effective at operator level**: bundled binaries and marketplace cache at 1.0.0-rc.23 still embed 10M; takes effect only after rc.24 is cut. 20M does not eliminate exhaustion for large cycle artifacts (`decision-log.md`, `burst-log.md`, `lessons.md`); size budgets and compaction remain the actual remedy. Compact at next cycle boundary OR after S-15.03 PRIORITY-A automation. |
| **Pseudocode-attested gate (META-LEVEL-24)** | burst-log Dim-2 contains `extract <foo>` narrative without literal shell command | Re-invoke gate via literal `grep -oE` / `diff` / `printf %s` with captured stdout per D-449(a); update Dim-2 with actual evidence |

### VSDD_SINK_FILE diagnostic capture (debug and release builds)

As of S-19.05 AC-004, `VSDD_SINK_FILE` is honored in both debug and release builds of the dispatcher. Set it to an absolute path before launching Claude to capture all observable plugin events as JSONL:

```bash
VSDD_SINK_FILE=/tmp/disp-events.jsonl claude --plugin vsdd-factory <args>
```

The file is appended-to on each dispatch; inspect it for `plugin.completed`, `plugin.abandoned`, `plugin.timeout`, and other BC-3.08.001 domain events. Internal lifecycle events (`internal.*`) are excluded.

**SEC-003 path constraint:** paths containing `..` (traversal sequences) or null bytes are rejected silently — a `tracing::warn` is emitted and no file is written. Use absolute paths from `mktemp` or a fixed `/tmp/` location.

### Step 4 — Re-run validators before re-dispatching

```bash
# Generic validator (if present at project root)
bash .factory/hooks/verify-sha-currency.sh 2>/dev/null || true

# Or run the registered validator scripts directly
for hook in plugins/vsdd-factory/hooks/*.sh; do bash "$hook" 2>&1 | head -20; done
```

Expected: exit 0 with `PASS` lines and no `FAIL` lines. If still failing, repeat Step 2 with the new dispatch's trace.

### Step 5 — Going-forward discipline (orchestrator)

To avoid the multi-commit-chain block:

- **Bundle backfills.** When state-manager performs multi-document backfills (e.g., adversary pass-N report + fix-pass-N closure report), stage all files THEN commit ONCE. Never two state-manager dispatches in a row both producing "backfill" commits.
- **Single-commit-per-burst.** Each logical burst (one adversary cascade step, one fix-pass cycle, one phase transition) → one commit in `.factory/`. Multiple consecutive commits with the same theme word (`backfill`, `Stage`) trigger the chain detector.
- **Soft-reset for recovery, never `--hard`.** The working tree state is what we want to preserve.
- **Force-push always needs user approval.** Per project git-safety protocol; orchestrator must request it from the human.

### Hook source locations (read-only reference)

- Dispatcher binary (operator-level cache): `~/.claude/plugins/cache/claude-mp/vsdd-factory/<version>/hooks/dispatcher/bin/<platform>/factory-dispatcher`
- Hook registry config (operator-level cache): `~/.claude/plugins/cache/claude-mp/vsdd-factory/<version>/hooks-registry.toml`
- Hook plugins WASM (operator-level cache): `~/.claude/plugins/cache/claude-mp/vsdd-factory/<version>/hook-plugins/*.wasm`
- Dispatcher source (this repo): `crates/` (build target → `target/release/factory-dispatcher`)
- Hook registry source (this repo): `plugins/vsdd-factory/hooks-registry.toml`
- Bash hook scripts (this repo): `plugins/vsdd-factory/hooks/*.sh`

## Tooling

- `compute-input-hash` (vsdd-factory plugin): `bin/compute-input-hash --scan .factory` for drift detection, `--update` to bump hashes after legitimate content changes
- `lobster-parse`: workflow file parser (`.lobster` YAML workflow files in `workflows/`)
- Factory orchestrator: invoked via `/vsdd-factory:run-phase <phase-id>` or per-skill slash commands
- Bats test runner: `cd plugins/vsdd-factory/tests && ./run-all.sh`

## Project References

| Path | Description |
|------|-------------|
| `.factory/STATE.md` | Live pipeline state (current phase, decisions log, Session Resume Checkpoint with 11 sections) |
| `.factory/policies.yaml` | Project governance policy registry (12 baseline policies; auto-loaded by adversary) |
| `.factory/cycles/v1.0-feature-engine-discipline-pass-1/` | Active F5 cycle artifacts: decision-log.md (D-NNN), lessons.md (L-EDP1-NNN), burst-log.md, INDEX.md, adv-cycle-pass-NN.md |
| `.factory/specs/architecture/` | Architecture docs + ADRs + ARCH-INDEX.md (subsystem registry) |
| `.factory/specs/behavioral-contracts/` | BC files + BC-INDEX.md |
| `.factory/specs/verification-properties/` | VP files + VP-INDEX.md |
| `.factory/specs/domain-spec/` | L2 domain spec (entities, invariants, capabilities, edge cases) |
| `.factory/stories/` | Per-story implementation specs + STORY-INDEX.md |
| `.factory/logs/dispatcher-internal-YYYY-MM-DD.jsonl` | Daily dispatcher event log (search by trace UUID) |
| `plugins/vsdd-factory/` | Marketplace plugin source (agents, skills, hooks-registry.toml, hook scripts) |
| `plugins/vsdd-factory/tests/run-all.sh` | Bats integration test runner |
| `crates/` | Rust workspace (dispatcher binary + hook-sdk + context-resolvers + supporting crates) |
| `workflows/*.lobster` | YAML workflow definitions parsed by lobster-parse |
| `bin/lobster-parse` | Workflow file parser CLI |
| `RELEASING.md` | Canonical release procedure (consulted by `/vsdd-factory:release` skill) |
| `docs/dispatch-package-authoring.md` | Dispatch package authoring requirements — mandatory dependency verification discipline (TD #74) |
| `CHANGELOG.md` | Release history |
| `.github/workflows/ci.yml` | PR + push CI: fmt + clippy + cargo test + bats |
| `.github/workflows/release.yml` | Release automation (cross-platform binary builds + marketplace publish) |
| `.github/workflows/release-branch-guardrail.yml` | TD #69 enforcement: release branches MUST target main |

## When in Doubt

If you are an AI agent and you are uncertain whether the production-grade default applies in a specific case, the answer is YES. The principle is the default. Ask only if you have a concrete reason to suspect this case is an exception.

If you are a human reviewing this file and you want to change the principle, edit this file and commit. The principle becomes whatever this file says.

## See also

- [`RELEASING.md`](./RELEASING.md) — canonical release procedure
- [`CHANGELOG.md`](./CHANGELOG.md) — release history
- [`.factory/STATE.md`](./.factory/STATE.md) — pipeline state (live; canonical post-clear/post-compact resume source)
- [`.factory/policies.yaml`](./.factory/policies.yaml) — governance policy registry
- [`.github/workflows/release.yml`](./.github/workflows/release.yml) — release automation
- [`.github/workflows/release-branch-guardrail.yml`](./.github/workflows/release-branch-guardrail.yml) — TD #69 enforcement
