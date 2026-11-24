# Review of the seams between the five 1.0 designs

Reviewer lens: names, paths, schemas, identifiers and responsibilities that cross from one design to
another. Every "Interfaces I need" row in each document was checked against the "Interfaces I provide"
of the others. Written 2026-11-23 against `pipeline.md`, `state.md`, `agents-skills.md`, `hooks.md` and
`packaging.md` as they stood on disk.

## Verdict

The five designs agree on the big shape (one state directory on its own branch, a stdlib Python runner
behind one launcher, workflows that steer and a tool that measures, walls keyed on `agent_type`), and
many small interfaces line up exactly. The seams do not hold yet. Seven problems would stop the system
working as written. The measuring commands that every gate and merge depends on are owned by nobody.
The workflows run agent types the roster does not have, including the `clerk` that opens and closes
every run. Step prompts leave out the inputs that agent files are told to stop without. The red
evidence lookup is defined at the current commit, so tests-first blocks the implementer after its
first commit. A skill and a workflow both claim `/vsdd-factory:spike`. And pipeline/hooks and state
build multi-repo on two incompatible storage models. Behind those sit 26 major gaps, most of them
"both disclaim" jobs (the decide command, the driver, cycle opening, a backlog, a diff producer,
deletion) or guarantees that one design gives and another undoes (config writable by tools, an
ungoverned `vsdd pr-merge`, walls bypassed through the plugin's own CLI and through browser `file:`
URLs). None needs a new architecture. Each needs one owner to decide and the others to adopt the
decision, and the synthesis should do that before any build package starts.

Finding counts: 7 blockers, 26 major, 11 minor.

---

## Blockers

### SEAM-01. The gate, delivery and verdict commands have no owner

- Severity: blocker
- Documents: pipeline §2.2, §2.9, §4 I8, §5.4 P1, risk R7; state §2.8, §2.16; packaging §2.10, §2.12;
  hooks §2.7 (`gates.toml`), §2.11.4, §2.11.7.
- Problem: pipeline makes these subcommands the enforcement backbone and asks packaging for them (P1):
  `gate <red|green|demo|spec|plan|baseline|wave|ui|release|formal|ingest|audit|contracts|integration>`,
  `story deliver`, `story new-fix`, `review verdict`, `holdout env`, `release publish`. Packaging's
  command table (§2.10) lists none of them, and its build plan (§2.12) has no package for them. State
  §2.16 hosts only the commands pipeline marks P (survey, checkpoint, decide, plan schedule, spike,
  idea register, report debt) and says it "supplies the stores they read and write". State §2.8 and
  hooks §2.11.4 name evidence writers (`vsdd gate`, `vsdd record-test`) that no command table defines.
  The `step record` fallback (pipeline R1, hooks §2.11.7) has no home either. Survey and checkpoint are
  specified in state §2.16 but appear in no build package: packaging S1 and S2 cover only the layout,
  status, briefing, flush and lock.
- What breaks: nothing writes the evidence that `tests-first`, `merge-guard` and delivery rely on.
  Every `gated()` step fails, and nothing can merge. Pipeline R7 calls this the critical component.
- Resolution: give the whole measuring tool to one owner. The state package is the natural home,
  because survey, checkpoint and the stores already live there. Add a section that specifies each
  subcommand: its arguments, its JSON output (the pipeline §2.10.6 and §2.10.8 shapes), the evidence
  record it writes and its exit code. Settle `record-test` (drop it or define it) and `step record`.
  Add build packages for gates, delivery and verdict to packaging §2.12, ahead of the W packages.

### SEAM-02. The `clerk` agent does not exist, and its tier breaks the agent lint

- Severity: blocker
- Documents: pipeline §2.1 (Tiers), §2.11.1 `relay()`, §5.2 A3, §6 q7; agents-skills §2.3, §2.4, §2.12
  roster, §4 P-8, §5 N-PIPE-2; hooks §2.7 `tool_only`, §5 P2; packaging §2.6 L-AGENT and roster
  `[tiers]`.
- Problem: every workflow's first and last step (survey and checkpoint), and every gate and delivery
  relay, runs `agentType: 'vsdd-factory:clerk'` on Sonnet at low effort. Agents-skills has no clerk.
  Its N-PIPE-2 says mechanical steps use `devops` and that every step sets `effort: 'xhigh'`. Its lint
  (P-8) requires `effort: xhigh`, and packaging's roster has only three tiers. Hooks implements H8 as
  `tool_only` on a clerk row that `walls.toml` does not contain.
- What breaks: every workflow dies at its survey on an unknown agent type, and packaging's universal
  test 3 fails. Using `devops` instead loses the tool-only restriction, because devops needs a general
  shell. It also puts a full devops system prompt, which demands a method name, behind a prompt that
  says "run this one command".
- Resolution: add `clerk` as a 19th agent: tools `Bash, StructuredOutput`, a body of about 15 lines.
  Add a fourth tier, `clerk`, to `roster.toml` and agents-skills §2.3, and let the lint allow
  `effort: low` for that tier only. Give it a `walls.toml` row with `tool_only = true`,
  `read_deny = ["holdout", "reviews", "runs"]` and `write = []`. The question of low effort (pipeline
  open question 7) stays with the user.

### SEAM-03. Workflow scripts name agent types the roster merged away

- Severity: blocker
- Documents: pipeline §2.9.x stage tables, §2.11.1 `ROLE`, §2.11.2 (`data-engineer`), §5.2 A1;
  agents-skills §2.4, §3.1, §4 P-1; packaging §2.6 (universal test 3, L-DEAD, L-XREF); hooks §4.2
  (second mismatch).
- Problem: pipeline steps use `business-analyst` (the `domain` stage of `spec`, and also `discover`,
  `ingest`, `maintain` and `multi`), `research-agent`, `data-engineer`, `devops-engineer`,
  `dx-engineer`, `formal-verifier`, `e2e-tester`, `performance-engineer`, `dtu-validator`,
  `spec-reviewer`, `validate-extraction`, `accessibility-auditor`, `visual-reviewer` and
  `session-reviewer`. The roster has 18 different names. A1 says "the scripts take the type from the
  ROLE table", but `ROLE` maps a type to a tier, not an old name to a new one. `e2e-tester` has four
  possible homes, so `ui-verify` (which runs `gate ui`) maps to no single agent.
- What breaks: `spec` cannot run its first stage. `ingest`, `discover`, `maintain`, `verify`, the
  wave's UI checks and the retrospective all fail on unknown types. Universal test 3 and L-DEAD fail.
  Any unlisted type falls to hooks' `other` row, which can write nothing in the state directory.
- Resolution: pipeline adopts the 18 roster names in every stage table and skeleton. Examples:
  `business-analyst` becomes `product-owner` for domain and project context, and `researcher` for
  synthesis and idea briefs. `session-reviewer` becomes `researcher` with method `retrospective`.
  `ui-verify` becomes `verifier` or `demo-recorder`; pick one. Generate the prelude's `ROLE` table from
  `roster.toml` (SEAM-15), so that a name outside the roster cannot be written.

### SEAM-04. Step prompts leave out inputs that agent files require, so agents return blocked

- Severity: blocker
- Documents: pipeline §2.5 ("What every step is given"), §2.11.1 `brief()`, §2.11.2 and §2.11.3
  prompts; agents-skills §2.7 (the Inputs rule and the adversary example), §2.8, §5 N-PIPE-3; state
  §2.5 (review report), §2.16 (`vsdd review path`).
- Problem: every agent file says "Your task names a method" and "If a required input is missing,
  return blocked; do not look for a substitute". The adversary's required inputs are a method name,
  the target paths, the evidence paths, a report path and the review rules. Pipeline prompts give a
  task sentence, the worktree, the story path and the contract ids. They never give a method name, a
  report path or `review-rules.md`. Several pipeline steps have no method in agents-skills §2.8 at all:
  `arbitrate` (architect), the scoring in `discover` (product-owner, architect, adversary), and the
  security audit and gap analysis in `ingest`. Reviewers have no shell, so they cannot call
  `vsdd review path` themselves.
- What breaks: a reviewer that follows its file returns `blocked` on the first pass. `review()` then
  returns `ok: false`, `converge()` returns `failed`, and every story and wave fails. Without report
  paths, state's review reports are never written.
- Resolution: give `step()` a required `method` option, which `brief()` writes as `Method: <name>`.
  Add a universal workflow test that every step names a method from agents-skills §2.8. Have the
  survey return, for each unit and lens, the report path (computed by the same code as
  `vsdd review path`) and the path of the review rules, and put both in every reviewer prompt. Add the
  methods `arbitrate` and `score-ideas`, and assign the `ingest` audits as SEAM-18 describes.

### SEAM-05. Red evidence is looked up at the current commit, so tests-first blocks the implementer after its first commit

- Severity: blocker
- Documents: state §4 (answer to S7: `evidence(project, gate, subject)` "returns the passing record for
  the subject's current commit"), §2.8 (evidence path `<gate>.<sha12>.json`); hooks §2.11.4
  (`tests-first`), §5 S7; pipeline §2.9.1 (red gate).
- Problem: hooks needs "a passing red record for the story", with its list of frozen tests, for as
  long as the story is open. State defines the function at the current commit only. The red gate
  passes on the test-writer's commit, and the implementer's TDD loop then commits once per test.
- What breaks: after the implementer's first commit, the head has no red record. Every further source
  edit is blocked with `no_red_evidence`, and the frozen-test check loses its list. The reverse also
  happens: after a later commit, the test-writer's `source:before-red` grant opens again. The implement
  stage cannot finish.
- Resolution: specify lookup per gate in state S7. For `red`, return the latest passing red record on
  any commit in `base..head` of the story branch; that record carries the hashes of the frozen tests.
  For `green` and `demo`, return the record for the head commit. Add two hooks cases: "the implementer
  edits after a commit" and "the test-writer edits source after red, at a later commit".

### SEAM-06. A skill and a workflow both claim `/vsdd-factory:spike`

- Severity: blocker
- Documents: pipeline §2.3, §2.8 ("the slash command is a workflow and not a skill"), §2.9.12, §4 I12,
  §5.2 A5 ("No model-visible spike skill"); agents-skills §2.11, §2.12 (a model-visible `spike` skill),
  §4 P-4, P-8 ("no skill has a workflow's name"), P-9, `model_visible_skills = 2`; `rewrite/STATE.md`
  (the table of spike entry points).
- Problem: both designs ship `spike` in the same namespace, and each rejects the other's form in so
  many words. The named-spike operations (`open` and `close`) exist only in the workflow.
- What breaks: whichever is built second fails the lint. If both ship, the command resolves to one of
  them unpredictably. The skill budgets (one model-visible skill or two) and the probes disagree.
- Resolution: choose one. Recommendation: the workflow. It shares `runSpike()` with the wrapper, puts
  the limits and the ledger in code, and carries named spikes. Drop the spike skill and set
  `model_visible_skills = 1`. The main session starts the workflow, or dispatches `spike-runner`
  directly as the shared agent block already allows. Update the entry-point table in
  `rewrite/STATE.md`. If the user wants a model-visible skill, rename the workflow (`spike-run`) and
  have the skill start it.

### SEAM-07. Multi-repo storage: one state per repository, or one hub

- Severity: blocker (for the multi-repository capability)
- Documents: pipeline §2.9.11 ("Each repository is an ordinary factory project with its own state"),
  §2.9.7, §5.3 H7; hooks §2.5 (`project`), §2.8 ("a repository … that has its own state is governed by
  that state"); state §2.14 (all state in one hub `.factory/`; member repositories hold nothing), §2.7
  (one `cycle`), §2.12 (`layout = "hub"`, `[[repo]]`).
- Problem: pipeline and hooks build a multi-repository project as N factory projects plus a root
  manifest. Each has its own status, cycle, decisions and identifier space. State builds one state
  directory with one identifier space, stories that carry `repo`, and no manifest apart from
  `factory.toml`.
- What breaks: under state's model, `multi`'s survey ("each repository's state and next"), the
  per-repository release order and per-repository decisions have nothing to read. Under pipeline's
  model, state's single `status.json`, its `[[repo]]` table and its global identifiers are wrong. The
  two build packages would produce incompatible code for a capability the user required.
- Resolution: adopt state's hub model. It keeps the single-state-directory rule that the walls, the
  briefing and `write-scopes` check 5 depend on. Rewrite pipeline §2.9.11 and hooks §2.8 to match:
  `repo` selects a `[[repo]]` entry for paths, commands and branches; `multi`'s survey reads progress
  per repository from each story's `repo` field; release order follows story dependencies; there is
  one cycle. The alternative is to make state per-repository and add a project-level store.

---

## Major

### SEAM-08. `factory.toml` can be written by tools in state, and only by code in hooks

- Severity: major
- Documents: state §2.4 class table (`config`: write `tools`; writer the main session through `setup`;
  "gate-guard protects `[hooks]`"), §2.12 ("People and the `setup` skill write it"); hooks §2.6 ("the
  project configuration file is in the `control` path class, which no tool call may write"), §2.7
  (`classes.toml` example), §2.11.10 (`gate-guard`, which has no `[hooks]` check), `walls.toml` `main`
  row; agents-skills §2.9 (`control`), §2.12 (`setup`).
- Problem: hooks' promise that only a person can relax a gate rests on `factory.toml` being code-only.
  State, which owns the content of `classes.toml`, makes it writable by tools and hands protection of
  `[hooks]` to `gate-guard`, which does not implement it.
- What breaks: built from state's table, the main-session model can set
  `[hooks.rules.tests-first] strength = "off"` with Edit. Built from hooks' table, the `setup` skill
  cannot write the configuration it is specified to write.
- Resolution: give class `config` `write = "none"` for tool calls. A person edits the file, and
  `vsdd init` creates it. The `setup` skill shows the detected values and tells the person what to
  change, or `vsdd init --set <key>=<value>` writes keys outside `[hooks]` from code. If tool edits
  are kept, `gate-guard` must compare the old and new content and refuse any change under `[hooks]`,
  with a case in its table.

### SEAM-09. Three designs claim story delivery, and `vsdd pr-merge` is an ungoverned merge route

- Severity: major
- Documents: pipeline §2.9.1 (`factory story deliver`), §5.3 H4 (G2); packaging §2.10 (`vsdd pr-merge`,
  `vsdd worktree`), §5 H5, P5; agents-skills §2.4 (devops: "pull requests, CI, merges"), §2.8 (devops
  methods `worktrees`, `gates`, `pull-request`); hooks §2.7 (`gates.toml` `[governed]`), §2.11.1 ("A call
  of the plugin's own tool is in no family"), §2.11.4 (`merge-guard`), §4.2 (first mismatch).
- Problem: pipeline delivers through one tool command that a clerk runs. Packaging ships `pr-merge` and
  `worktree ensure|remove` as public subcommands and asks hooks to name `pr-merge` as the governed
  route. Agents-skills gives devops methods for pull requests, merges and worktrees. Hooks exempts every
  `vsdd` call from the command families and lists only `story deliver` and `release publish` as
  governed.
- What breaks: any agent with a shell can run `vsdd pr-merge <pr> --reviewed-sha <sha>`. That command
  checks only that the head matches, so it merges without green evidence, a converged verdict or the
  demo gate, which defeats G2. Devops's merge method is blocked by `merge-guard`. Two worktree creators
  can disagree on paths.
- Resolution: `vsdd story deliver` and `vsdd release publish` are the only subcommands that merge.
  `pr-merge` and `worktree` become library functions that they call, or stay as subcommands that
  `gate-guard` refuses for every agent caller and that `gates.toml` lists. Remove the devops methods
  `pull-request`, `worktrees` and `gates`, and the word "merges" from its role. The answer to packaging
  P5 is "no".

### SEAM-10. Walled agents can read walled content through the plugin's own CLI

- Severity: major
- Documents: state §2.6, §2.16 (`vsdd list`, `refs`, `coverage`, `path`, `config`, `status`); hooks
  §2.11.1, §2.11.2 (the Bash rule); agents-skills §2.5 (holdout-evaluator, verifier, spike-runner,
  implementer and test-writer have a shell), §2.9.
- Problem: `read-walls` looks for paths in Bash tokens. `vsdd list holdout`, `vsdd refs BC-2.03.007`
  (which can list the scenarios that cite a contract) and `vsdd list story` print walled content
  without naming a path. The tool cannot see the caller's agent type, and hooks treat every `vsdd`
  call as allowed.
- What breaks: builders can read hidden-scenario titles and the contracts each scenario covers (wall
  W1). The holdout evaluator can read stories and contract titles (wall W2).
- Resolution: add a `[reveals]` table to `gates.toml` that maps each read subcommand, and its kind
  argument, to the path classes it prints. `read-walls` then treats a `vsdd` call as a read of those
  classes for the caller's row. As a second line, the tool prints holdout and reviews content only when
  given an explicit flag, which `read-walls` checks.

### SEAM-11. Browser tools read local files outside the walls, and the two lint rules contradict each other

- Severity: major
- Documents: agents-skills §2.5 (BROWSER for holdout-evaluator, demo-recorder and ui-reviewer), §5
  N-HOOK-9; hooks §2.3 ("MCP tools … never reach it"), §5 A2 (a walled agent may hold only tools the
  runner can inspect, or tools that cannot read project files), §4.2 (N-HOOK-9 is not answered).
- Problem: Playwright's `browser_navigate` to `file:///…/src/…` reads any file. Hooks never sees MCP
  calls, and its A2 lint would reject the holdout evaluator's tool list.
- What breaks: the holdout evaluator, whose isolation is the whole point of hidden scenarios, can read
  source and tests. Whichever lint is built fails against the other design.
- Resolution: add both spellings of Playwright's `browser_navigate` (and any other tool that opens a
  URL) to the `PreToolUse` matcher, and have `read-walls` treat a `file:` URL as a Read of its path.
  Teach the A2 lint that these tools are inspected. Extend the Bash heuristics to `npx playwright` with
  `file:` URLs.

### SEAM-12. Nobody can review the hidden scenarios

- Severity: major
- Documents: pipeline §2.9.3 (scenarios: "Only product-owner, spec-reviewer and holdout-evaluator can
  read them"; review by spec-reviewer across the set), §5.3 H3 (W1); agents-skills §2.4 (spec-reviewer
  merged into adversary), §2.9 (adversary denied `holdout`), §2.2 rule 1.
- Problem: pipeline relies on spec-reviewer to review the scenarios at the specification perimeter. The
  roster folds spec-reviewer into the adversary, which is walled from `holdout`. The holdout evaluator
  is walled from the contracts it would check the scenarios against.
- What breaks: the scenarios that drive the wave and release gates, including the must-pass floors, are
  written and never independently reviewed. A wrong scenario produces fix stories that cannot succeed.
- Resolution: add a `scenario-reviewer` agent on the reviewer tier. It reads `holdout` and
  `spec-product`, creates review files only, is never used on code and is never handed to builders.
  Agents-skills rule 1 justifies it, because its wall differs from every other agent's. The weaker
  alternative is to have the holdout evaluator review the scenarios against the brief only, and to say
  what that leaves unchecked.

### SEAM-13. Pipeline's re-review prompts carry builder claims and earlier findings, which agents-skills forbids

- Severity: major
- Documents: pipeline §2.7 (step 1 of the loop: "From the second pass they receive the list of claims"),
  §2.11.1 `converge()` (claims include the fixer's `note`), §2.11.2 (review prompt), §2.11.3 (`carried`
  deferred findings); agents-skills §2.9 ("Three rules for prompts": never a builder's summary or an
  earlier finding), §2.7 (adversary: "You have not seen any earlier review").
- Problem: the convergence rule needs claim checks. The reviewer contract forbids them.
- What breaks: if reviewers are built to the agents-skills text, claim checks come back empty.
  `converge()` then never sees `resolved` or `withdrawn`, disputes never close, and loops run to the cap.
  If prompts include the claims, they break the fresh-context rule, and the prompt-rule test in
  agents-skills risk 3 fails.
- Resolution: make a claim structured data only (key, location, resolution, commit). Remove the fixer's
  free-text note, or cap it and label it as the builder's position. State this exception in
  agents-skills §2.9 and in the adversary file. Pass deferred findings to the wave reviewer as "scope to
  check", not as conclusions; they come from reviewers, not builders.

### SEAM-14. Severity, the routing field, the key and the role fields differ in the finding contract

- Severity: major
- Documents: pipeline §2.7 (severity table), §2.10.2 (finding schema), §2.10.8 (review and holdout
  data), §2.9.3 ("findings carry the owning role"), §5.2 A7; agents-skills §2.6 (returns: `owner`,
  `out_of_scope`, `not_reviewed`, `counts`, `score`), §2.7 (adversary severities), §5 N-PIPE-4.
- Problem: the adversary file defines medium as "will mislead a builder" and high as "a requirement
  unmet". Pipeline's high includes "an edge case untested" and "a test cannot fail", and its medium is
  "a real defect no contract covers". Agents-skills routes fixes on `owner`; pipeline's schema has no
  `owner` and routes on `category`, and spec-stage fixes "by the owning planner" have no field to read.
  The key format `<category>:<path>:<symbol>` appears only in pipeline. Field names differ: `score`
  against `satisfaction`, `out_of_scope` against `scope`, `not_reviewed` against `coverage`.
- What breaks: which findings block depends on definitions the reviewer never reads. Spec-stage fixes
  cannot be routed. Keys that are not stable break claim matching and the repeat signal at the cap.
- Resolution: pipeline's §2.10 schemas and §2.7 definitions are the contract. Copy the severity table
  word for word into every reviewer file, checked by lint. Add `owner` (an enum of roster names) to the
  finding schema, and route both spec and story fixes on it. Put the key rule into the reviewer files.
  Rewrite agents-skills §2.6 role fields in the schema's field names.

### SEAM-15. Model and effort have two sources of truth, and they already disagree

- Severity: major
- Documents: pipeline §2.1 (Tiers), §2.11 ("every step sets model and effort itself"), §2.11.1 (`TIER`,
  `ROLE`, `runSpike`); agents-skills §2.3 ("The agent file is the single source of truth for the model.
  Workflow steps therefore do not set `model`"), §2.4, §5 N-PIPE-2; packaging §2.6 (roster `[tiers]`,
  L-AGENT).
- Problem: the prelude hard-codes a `ROLE` table that overrides agent frontmatter on every step. It
  already differs from the roster. Spike-runner is builder in `ROLE` and planner in the roster.
  `runSpike()` picks planner for research and builder for experiments. Session-reviewer is reviewer in
  `ROLE`, while the roster moves the retrospective to `researcher`, a planner. The clerk runs at low
  effort.
- What breaks: user decision 4 is enforced by lint on the agent files and then silently overridden at
  run time. Tuning a tier "in one line of the file" (agents-skills risk 9) has no effect inside
  workflows.
- Resolution: keep one source. Either generate the prelude's `TIER` and `ROLE` from
  `tests/lint/roster.toml` in the packaging build step, or follow agents-skills and set only effort on
  steps. The user settles the spike-runner's tier (agents-skills open question 1).

### SEAM-16. Promised generated files have no generator, and drift is already visible

- Severity: major
- Documents: pipeline §2.10 ("the single source for the scripts, the tool and the hook runner"), §5.4 P3,
  P4, §2.11.1 (schema literals); packaging §2.5 ("Two things are generated"; "Nothing else is built"),
  §2.6; hooks §5 K3, §2.3.
- Problem: pipeline asks for the JSON Schemas to be stamped into the prelude (P3), and for conformance
  fixtures that run the JavaScript and Python verdict and route code against the same cases (P4). Hooks
  asks for `hooks.json` to be generated from `registry.py` by `tools/build.py` (K3). Packaging generates
  only the prelude copy and the reference pages, and has no test layer for P4. The hand-written literals
  already differ from the schema files: `S_GATE` lacks `ok`, which `gate-result` requires, and
  `S_FINDING` requires 5 fields where the schema requires 10.
- What breaks: the script, the tool and the hook runner validate different shapes. `hooks.json` can
  drift from the rule table, which is how the old pre-compaction hooks went unwired. The JavaScript and
  Python verdicts drift (pipeline R2) with nothing to catch it.
- Resolution: add three outputs to `tools/build.py` in packaging §2.5, all covered by `--check`: schema
  literals generated from `<plugin>/schemas/*.json` into the stamped region, and `hooks/hooks.json`
  generated from `registry.py`. Add a conformance layer to packaging §2.6: shared JSON fixture cases run
  through `judge()`, `converge()` and the route rule in Node, and through the Python implementations.

### SEAM-17. Hooks never answered the needs state placed on it

- Severity: major
- Documents: state §5 H-1 to H-5, §2.3 (conditions), §2.11; hooks §4.2 (answers only pipeline,
  agents-skills and packaging), §2.3 (budgets), §2.8 (conditions), §2.10, §2.11.3.
- Problem: hooks was written before state existed, and it implements none of these:
  - H-3: `code_values`, so that only code may set `approved`, `ready`, `done` or `proven`.
  - H-4: refuse state writes while the mount is broken.
  - H-5: refuse reads of `.factory` inside a linked worktree.
  - H-2: refresh the lock's `seen` on agent start and stop, and release the lock at `SessionEnd`.
  - H-1: flush on `SubagentStop` and `SessionEnd`. This one conflicts with hooks' 1-second deadline
    for notify events and its limit of one git subprocess per rule.

  The two designs also define the "partial" condition differently.
- What breaks: an agent can write `status: approved` into a contract and skip decision `spec`. A step
  that runs 20 minutes without calling `vsdd` makes its lock look stale, and a second session takes over
  a live run. State can be written into a directory that is not mounted.
- Resolution: add H-2 to H-5 to hooks §2.10 and §2.11, each with cases. That means the `code_values` and
  `dir` checks in `artifacts`, the broken-mount block in `write-scopes`, the read block for `.factory`
  inside a worktree, `lock_seen` in `lifecycle-log`, and `lock_release` on `SessionEnd`. Adopt state's
  five-condition `detect()` as the one definition. Drop H-1: flush only on `PreCompact` and inside `vsdd`
  commands, which state itself says loses nothing.

### SEAM-18. Builder work outside a story has no worktree, no branch and no route to the integration branch, and some outputs have nowhere to go

- Severity: major
- Documents: pipeline §2.9.5 (readiness: CI and toolchain), §2.9.6 (formal, performance, ui), §2.9.7
  (prepare), §2.9.8 (synthesize: security audit, verification gap analysis, project context), §2.9.9
  (idea briefs), §2.9.10 (sweeps); hooks §2.7 (`story_only`), §2.11.2 check 6, §2.11.4 (`merge-guard`);
  agents-skills §2.9 (write scopes); state §2.3 (the person's checkout is never committed to), §2.4.
- Problem: these steps write or commit product files, but they are not stories. With `story_only` they
  can write nothing. Without it they write in the person's checkout. Either way `merge-guard` stops
  their commits from reaching the integration branch. Ingest's security audit has no class its writer
  may use: the security-reviewer may only create reviews, and the reviews path needs a cycle. The
  verification gap analysis has the same problem, because the verifier may write tests only. Idea
  briefs have no home at all.
- What breaks: plan readiness never produces CI. Verify cannot write its formal harnesses. The
  docs-drift sweep cannot fix anything. Ingest's baseline is incomplete.
- Resolution: adopt one rule: every change to the product tree is a story. Readiness becomes chore
  stories in `E-0`; harnesses and benchmarks become chore or fix stories; sweeps already produce fix
  stories. Steps that only measure write evidence through `vsdd` and nothing else. For ingest, give the
  audit and the gap analysis classes, for example `analysis/<scope>/security.md` with the
  security-reviewer granted `analysis:create`, and an architect assessment of kind `verification`. Put
  idea briefs under `research/`.

### SEAM-19. A named-spike step cannot write in its own worktree

- Severity: major
- Documents: pipeline §2.8 (named spikes), §2.9.12; hooks §2.7 (`own-worktree`), §2.11.6 (containment);
  agents-skills §2.7 (`isolation: worktree` on spike-runner); state §2.5 (the named spike's ledger
  files).
- Problem: the named-spike step must work in `.worktrees/spike-<name>/`. But spike-runner always gets
  its own platform worktree from its frontmatter, and hooks define `own-worktree` as the worktree that
  holds the caller's working directory. If isolation does not apply, the working directory is the main
  checkout, and every write is blocked.
- What breaks: `/vsdd-factory:spike {open: …}` can neither advance a named spike nor update
  `.spike/STATE.md` and `.spike/TASKS.md`.
- Resolution: hooks define `own-worktree` for the spike row as the worktree recorded in the caller's
  open ledger entry (`spike_entry`), falling back to the worktree of the working directory.
  `vsdd spike open --name` records that path. Add a hooks case.

### SEAM-20. The spike ledger and the spike finding differ from design to design

- Severity: major
- Documents: pipeline §2.8, §2.10.5 (the verdict enum includes `answered`), §2.11.1 (`runSpike()` runs
  research spikes without isolation); state §2.5 (ledger verdicts without `answered`), §2.18;
  agents-skills §2.6 (spike-runner returns `result` and evidence paths), §2.7 (Method), §2.9 (write
  scope: "its own entry in `spikes`"); hooks §2.11.6 (entries matched by worktree; every call blocked
  before `spike open`), §5 P5.
- Problem:
  - (a) A research spike returns `answered`, which `vsdd spike record` refuses when it validates
    against state's registry.
  - (b) Agents-skills has the runner write its ledger entry with the Write tool, but state and hooks
    make `spikes` code-only.
  - (c) The runner's result fields differ from pipeline's schema.
  - (d) Research spikes all use the main checkout as their worktree key, so `spike-bounds` can match
    the wrong entry.
  - (e) The agent file's first act, reading its method file, is blocked until `spike open` has run.
- What breaks: research spikes cannot be recorded, so the runner is refused when it tries to stop.
  Parallel research spikes match each other's entries. Spikes dispatched directly by an agent produce a
  second format.
- Resolution: use one verdict enum (add `answered` to state's registry). Write the ledger only through
  `vsdd spike open` and `vsdd spike record`: remove `spikes` from the runner's write scope and use the
  commands in the spike method file. Make the spike-runner's Result section the §2.10.5 schema. Isolate
  every spike, dropping pipeline's research exception, so the worktree key is unique. Let
  `spike-bounds` allow Reads under `${CLAUDE_PLUGIN_ROOT}/methods` and `/templates` before `spike open`.

### SEAM-21. Decisions and reviews need a cycle that most workflows never open

- Severity: major
- Documents: state §2.3 (layout: `cycles/<cycle>/decisions/`, `reviews/<cycle>/<unit>/`), §2.5, §5 P-8;
  pipeline §2.6 (decisions `ideas`, `finding` from `maintain`, `spike-limit`, `stuck`), §2.9.7 to §2.9.10.
- Problem: decision and review paths are scoped to a cycle, but `ingest`, `discover`, `maintain` and
  spikes outside a cycle all open decisions or write reviews. State P-8 asks pipeline to call
  `vsdd cycle open` at the first checkpoint and `vsdd cycle close` in `release`. Pipeline never mentions
  cycles.
- What breaks: decision `ideas`, a maintenance `finding` and the ingest review cannot be stored. Cycle
  pruning never runs, so the tree grows without bound.
- Resolution: state adds a standing home for records outside a cycle, either `cycles/_standing/` or a
  top-level `decisions/` and `reviews/<workflow>/`. Pipeline adds `vsdd cycle open` to the brief's
  checkpoint and to `feature`'s first checkpoint, and `vsdd cycle close` to the close stage of `release`.

### SEAM-22. A refused run leaves the lock held, and the hooks canary never runs

- Severity: major
- Documents: pipeline §2.5 (the survey takes the lock, and then "if a precondition is unmet the script
  returns at once"), §2.11.2 and §2.11.3 (early returns without a checkpoint); state §5 P-4, §2.11
  (staleness after 20 minutes); hooks §5 P6, §2.9 item 7.
- Problem: the survey takes the lock before it judges the preconditions, and the scripts return without
  calling `checkpoint`. Hooks P6's canary preflight appears nowhere in pipeline.
- What breaks: each refused run blocks the next command for 20 minutes with "a run lock is held by a
  live run". If hooks are switched off at the platform level, workflows run without gates and never
  notice.
- Resolution: the survey takes the lock only after the preconditions pass, or releases it when it
  returns `ok: false` or an unmet precondition. Every script return after a successful survey goes
  through `checkpoint`. To prove hooks are live, have `lifecycle-log` write a per-session marker on
  `SubagentStart`, and have the survey refuse when this session has none. That costs one file read and
  no extra step. The alternative is for the survey relay to run `vsdd hooks canary` first.

### SEAM-23. The `decide` command and the brief's checkpoint are missing from the skill roster

- Severity: major
- Documents: pipeline §2.6, §4 I12, §5.2 A6; hooks §2.11.5 (`UserPromptExpansion` needs a registered
  command), §5 A4, P4; agents-skills §2.12 (12 skills, none of them `decide`; the `brief` row), §4 P-9.
- Problem: the in-session way to settle a decision is a plugin command, `/vsdd-factory:decide`, that no
  design ships. The `brief` skill is specified as a conversation, with no `vsdd checkpoint brief` at its
  end. State's checkpoint takes `--run`, and a skill has no run to give it.
- What breaks: decisions can be settled only from a terminal, and `vsdd` is not on the terminal's PATH
  (SEAM-25). Decision `brief` is never opened, so the greenfield route never starts.
- Resolution: add `decide` to the roster as a person-only skill with `disable-model-invocation: true`.
  Its body tells the model to relay the hook's context and never to run `vsdd decide`. Update P-9 and
  `roster.toml`. End the `brief` skill with `vsdd checkpoint brief`, specified in state §2.16 as a
  checkpoint that takes no run and opens the cycle and the decision.

### SEAM-24. The driver has no skill and nothing writes its record

- Severity: major
- Documents: pipeline §2.4 (the driver), §5.2 A6, risk R4; state §2.3 (`local/driver.json`), §2.9 (the
  briefing line "The driver is active: continue it"); agents-skills §2.12 (no driver).
- Problem: low touch (user decision 7) depends on the driver. Agents-skills has no driver skill, and
  `local/driver.json` is code-only with no command that writes it.
- What breaks: a person has to type every workflow command. The briefing's "driver is active" line is
  never true, so a session that has been compacted never resumes driving.
- Resolution: add a person-invoked `drive` skill to agents-skills, running the loop pipeline §2.4
  specifies. Add `vsdd driver on|off` to state §2.16, writing `driver.json` with the session id. When
  the session id matches, the briefing tells the main session to continue.

### SEAM-25. `vsdd` is not on a person's terminal PATH, yet person-only operations are sent there

- Severity: major
- Documents: packaging §2.10 (`bin/vsdd` is on the Bash tool's PATH only), §2.11 (troubleshooting);
  pipeline §2.6 ("A person can also run `factory decide` in their own terminal"); state §2.11
  (`vsdd lock release`); hooks §2.2 (`Run: vsdd doctor`), §2.11.5, §2.12 (`vsdd explain`).
- Problem: no design puts `vsdd` on the person's own shell PATH, and the plugin's cache path changes
  with every version.
- What breaks: the fallback for decisions, the fix line printed on every launch failure, and the
  crash-recovery command cannot be run as written. When `uv` is missing, the person cannot run
  `vsdd doctor` from anywhere.
- Resolution: packaging adds a supported route. One option: `vsdd doctor --install-shim`, run once
  through the Bash tool, writes `~/.local/bin/vsdd`, which finds the active plugin path in
  `installed_plugins.json`. Add a conformance probe for whether `! vsdd …` typed inside Claude Code gets
  the plugin's `bin/` on PATH. Every message that tells a person to run `vsdd` uses the supported form.

### SEAM-26. The CLI convention refuses `vsdd init`, and the subcommand surfaces disagree

- Severity: major
- Documents: packaging §2.10 (conventions: "outside an initialised project, everything except `doctor`
  and `obs` says so and exits 2"; exit codes 0, 1 and 2); state §2.16 (`init`, `import`, `stale`,
  `status --fix`), §2.2 rule 5; hooks §2.13 (`hooks canary` exits 3).
- Problem: `vsdd init` and `vsdd import` must run before a project is initialised, and packaging's
  convention refuses them. Other surfaces differ too. Packaging's `stale --update` contradicts state's
  rule 5, that nothing stored is updated after the fact. Packaging has `status --wave`, and state has
  `status --fix --json`. The canary's exit code 3 is outside the convention.
- What breaks: `setup` cannot initialise any project, and the exit-code test fails on the canary.
- Resolution: packaging lists the exempt subcommands explicitly: `init`, `import`, `doctor`, `obs`,
  `hooks selftest` and `--version`. Adopt state's `stale`, without `--update`. Use
  `status [--fix] [--json] [--wave]`. Add exit 3 to the convention, for the canary only.

### SEAM-27. The story-writer is told to run a gate but has no shell

- Severity: major
- Documents: pipeline §2.9.4 (story-draft: "The agent finishes with `factory gate plan --story`"), §2.2
  (consequence 1); agents-skills §2.5 (story-writer: READ, EDIT, SPAWN, RESULT).
- What breaks: every fast-route feature (trivial, bug fix, hotfix) blocks at story-draft.
- Resolution: run `vsdd gate plan --story <id>` as a clerk step after the story-writer, as `plan`'s
  schedule stage already does, with at most two fix rounds.

### SEAM-28. The designs list different story kinds

- Severity: major
- Documents: pipeline §2.9.1 (kinds `fix`, `facade`, `port`, `docs`, `data`), §2.11.2
  (`u.kind === 'data'`); state §2.5 (story `kind`: `feature`, `fix`, `facade`, `docs`, `chore`); hooks
  §2.7 (`exempt_kinds`).
- What breaks: the `artifacts` rule refuses a `port` or `data` story when it is written, so ports cannot
  be planned at all.
- Resolution: use one enum: `feature`, `fix`, `facade`, `port`, `docs`, `chore`. Drop `data`, since
  agents-skills gives migrations to the implementer, and remove the `data-engineer` branch from
  `story.js`.

### SEAM-29. Queued change requests have nowhere to be stored

- Severity: major
- Documents: pipeline §2.9.4 (survey: "the request is recorded in the backlog and the run returns"),
  §3.2 ("storage is the state designer's"); state §2.3 (layout), §2.7, §5 P-6.
- What breaks: a request that is not critical, made while a cycle is active, is acknowledged and then
  lost.
- Resolution: state adds a register that code writes, either `requests/CR-NNN.md` files or a capped list
  in `status.json`. The `feature` survey writes it, and the briefing lists it. Add a route-table row
  that starts `feature` with the oldest queued request when the cycle closes.

### SEAM-30. Artifacts that are meant to be deleted cannot be deleted

- Severity: major
- Documents: state §2.4 (Deletion), §2.5 (lessons are folded and deleted, and `vsdd validate` fails at
  31 adopted; deferred items are "deleted by `defer resolve`"; superseded research is deleted); hooks
  §2.11.2 check 4 (shell writes into the state directory, `rm` included, are blocked for every caller);
  agents-skills §2.12 (`defer`).
- What breaks: `defer resolve` cannot work. Lessons pile up until `vsdd validate` fails at a gate.
  Research cannot be superseded. Each case needs a person at a terminal.
- Resolution: add code subcommands such as `vsdd defer resolve <slug>`, `vsdd lesson retire <slug>`
  and `vsdd research retire <slug>`. They delete only in classes whose registry entry allows deletion,
  then flush. The skills, and the architect's step that folds lessons, call them.

### SEAM-31. Reviewers cannot get the diff they are told to review

- Severity: major
- Documents: pipeline §2.11.2 (prompt: "Review the diff `base...head`"); agents-skills §2.5 (adversary,
  security-reviewer and code-reviewer have no shell), §2.6 (code review inputs are "evidence paths (diff
  patch, test output)"), §2.12 (`review` skill), §5 N-PKG-2; state §2.8 (the evidence store has a slot
  for a `.patch`); packaging §2.10.
- Problem: no command writes a review diff, and neither the survey's story unit nor the wave gate data
  carries a patch path.
- What breaks: story and wave reviewers can read the head tree but cannot see what changed. The
  code-reviewer, which reviews nothing but a diff, has no input. The `review` skill's `code` and `diff`
  lenses cannot work.
- Resolution: the tool writes `local/evidence/<subject>/diff.<sha12>.patch`, the slot state already
  defines, at the survey and after each fix. The story unit and the wave gate data return `diff_patch`,
  and reviewers receive it as an evidence path. Add `vsdd review diff [base]` for the `review` skill.

### SEAM-32. PDF export and diagram rendering have no tool

- Severity: major
- Documents: agents-skills §2.12 (`export-pdf`, `author diagram`), §5 N-PKG-4; packaging §2.10 (no
  converter), §2.3 and L-PY (shipped Python uses the standard library only).
- What breaks: two shipped skills have nothing to run, which leaves user decision 2 ("port all of it")
  unmet for them.
- Resolution: packaging owns both. For example, `vsdd export pdf` drives a converter it finds on PATH
  (headless Chrome through the bundled Playwright, or pandoc), and `vsdd diagram render` renders
  Mermaid and Excalidraw through the same browser. `vsdd doctor` checks both. The alternative is for
  the user to drop them explicitly.

### SEAM-33. Finding paths have no defined base, so grounding checks the wrong tree

- Severity: major
- Documents: pipeline §2.10.2 (`location.path`, no base stated), §2.11.2 (reviewers review a story
  worktree); hooks §2.11.7 (grounding is "resolved against the step's worktree, then the project root").
- Problem: reviewers run from the main checkout and review `.worktrees/S-x/`. The step identity carries
  no worktree, so the hook cannot know which tree the step reviewed. It resolves `src/lib.rs` against
  the project root and reads the main checkout's older copy of the file.
- What breaks: correct findings are rejected as ungrounded ("quote not found"). The reviewer retries
  until the platform's limit, and the step fails. Alternatively, a stale match is accepted.
- Resolution: the schema states that `location.path` is relative to the root of the reviewed tree. Then
  either the step identity carries `tree` (an absolute path), or hooks map (workflow, subject) to the
  unit's worktree through a state function such as `unit_tree()`.

---

## Minor

### SEAM-34. The tool's name and the prelude's conventions differ

- Severity: minor
- Documents: pipeline (it writes `factory` throughout, including the command text inside the prelude's
  prompts), §2.11.1 (markers `// >>> prelude`; source `<plugin>/workflows/_lib/prelude.js`); packaging
  §2.5 (markers `// >>> stamped:prelude`; source `workflow-lib/prelude.js`, which is not shipped; every
  `.js` file under `workflows/` must carry markers).
- What breaks: the prompts tell agents to run a command that does not exist. A file at
  `workflows/_lib/prelude.js` fails `build --check`.
- Resolution: use `vsdd`, packaging's markers and `workflow-lib/prelude.js`. L-XREF checks every
  `vsdd <subcommand>` that appears in a workflow's prompt text.

### SEAM-35. Path-class names and globs differ across three designs

- Severity: minor
- Documents: agents-skills §2.9 (`holdout` = `.factory/holdout/scenarios/**`, `holdout-reports`,
  `evidence` = `.factory/evidence/**`, no `runs`, `config`, `decisions`, `ideas` or `local`); state §2.4
  (`holdout` = `.factory/holdout/**` including evaluations, `holdout-evaluations`,
  `.factory/local/evidence/**`, `runs`); hooks §2.7 (examples use `.factory/evidence/**`, and `control`
  includes `factory.toml`).
- What breaks: a wall can name a class that matches no path, and reviewers are not denied `runs`, which
  is wall W3. The product-owner's grant on `holdout` also covers the evaluator's write-once evaluations.
- Resolution: state's §2.4 table is the only content of `classes.toml`. Update agents-skills' walls to
  those names, deny `runs` to reviewers, exclude `holdout-evaluations` from the product-owner's write
  grant, and fix the hooks examples. The policy tests in hooks §2.14 already assert that every class
  named in a row exists.

### SEAM-36. Limit and configuration keys go by different names

- Severity: minor
- Documents: pipeline §2.11.1 (`RUN.limits` keys: `agents`, `stories_per_run`, `attempts`, `passes`,
  `rounds`, `spikes_per_step`, `spikes_per_run`, `spike_minutes`), §2.11.2 (merge
  `{...RUN.limits, ...s.limits}`); state §2.12 (`limits.agents_per_run`, `stories_per_wave_run`,
  `step_attempts`, `convergence.passes`, `spikes.per_step`, `spikes.minutes`; `spikes.max_per_session`
  and `max_per_requester`, attributed to hooks; `max_open`).
- What breaks: a project's limits are merged in under different names, so they are ignored without any
  error and the defaults run. The keys state attributes to hooks are never defined by hooks, and
  nothing enforces `max_open`.
- Resolution: the survey returns `limits` under the prelude's names, with the mapping written in state
  §2.12, and a universal harness test fails on any limit key it does not recognise. Drop the phantom
  keys, or give each one an enforcer.

### SEAM-37. Decision choices cannot carry their arguments

- Severity: minor
- Documents: pipeline §2.6 (`approve [variant=<id>] [topology=…]`, `waive <keys> "<reason>"`, a choice
  per idea for `ideas`); hooks §5 P4 (the id and the choice are the first two words; the rest is a
  note); state §2.8 (`settle_decision(id, choice, note)`; statuses `approved`, `revise`, `rejected`).
- What breaks: `variant`, `topology` and waiver keys end up in free text. Decision `ideas` cannot record
  a separate choice for each idea. Nothing says how `hold`, `direct` or `descope` map to a stored
  status.
- Resolution: define the grammar as `<id> <choice> [key=value …] [-- note]`, open one decision per idea
  (`ideas.ID-007`), and add a table in state that maps each kind's choices to statuses.

### SEAM-38. Agents-skills refers to workflows and jobs that no longer exist

- Severity: minor
- Documents: agents-skills §2.12 (assumed workflow names) and §3.2 (homes: `stories`, `holdout`,
  `refine`, `harden`, `converge`, `fix`, `multi-repo`; market intelligence as "the first gate of
  `spec`"), §2.8 (the story-writer's `wave-plan` method), §2.12 (`status` skill); pipeline §2.3; state
  §2.18, §5 A-1.
- Resolution: use pipeline's twelve names, and remove `wave-plan`, because `vsdd plan schedule`
  computes the waves. The `status` skill body follows state A-1, which includes saving a person's
  standing instruction with `vsdd note`.

### SEAM-39. Packaging's layout and lint leave out other designs' directories

- Severity: minor
- Documents: packaging §2.3 and the §2.6 lint table; agents-skills §4 P-7, P-8 (`<plugin>/methods/`, the
  shared block byte for byte, every named method must exist, method files at most 150 lines); pipeline
  §4 I2 and I4 (`<plugin>/pipeline/stages.toml`, `<plugin>/schemas/`); hooks §2.4 (`py/vsdd/hooks/`,
  `py/vsdd/policy/`); state §5 K-5 (lint skeleton templates against the registry).
- Resolution: add these directories to the layout, and add lint rows for method files, templates
  against the registry, the agents' shared block and method existence.

### SEAM-40. The stage registry and the route table are promised as data but not specified

- Severity: minor
- Documents: pipeline §4 I2 (`stages.toml` "as tabulated"), I3 (the route table, given in prose), §2.10.7
  (`next` is an object); state §5 P-1 (it needs input and output globs for each stage to compute
  digests), P-2 (the route table as data), §2.7 (`next` is a string).
- Resolution: pipeline specifies the schema of `stages.toml`, with globs for each stage, and a data file
  for the route table. Agree one shape for `next` and use it in `status.json` and in workflow results.

### SEAM-41. Two test and operations needs went unanswered

- Severity: minor
- Documents: packaging §5 P3 and §2.13 N9 (cheaper models for end-to-end runs; the prelude sets the
  model on every step, so environment overrides may not take effect); pipeline §5.4 P6 (a headless
  recipe for scheduled `maintain` and `discover`).
- Resolution: the step wrapper honours `args.test_tier`, or tiers generated from the roster carry a
  test override. Packaging documents the scheduled headless recipe in `docs/running-the-pipeline.md`.

### SEAM-42. Ancestor searches are blocked for every agent, not only the main session

- Severity: minor
- Documents: hooks §2.11.2 (a Grep or Glob whose root is an ancestor of a denied class is blocked "for
  every row except `main`"); state §2.3 and §4 (answer to N-STATE-8: `.factory/` in `.git/info/exclude`),
  risk 10.
- What breaks: planner and reviewer steps run with the project root as their working directory, so
  their default Grep is refused, and they have to narrow every search.
- Resolution: once the probe confirms that Grep honours `.git/info/exclude`, allow ancestor searches for
  every row whose denied classes all lie under `.factory/`. Until then, step prompts pass explicit
  search paths.

### SEAM-43. The holdout evaluator's findings cannot be grounded without leaking scenarios

- Severity: minor
- Documents: pipeline §2.10.2 (`location.path` is required), §2.10.8 (holdout findings are "symptoms
  only"), §2.9.2; hooks §2.11.7.
- Problem: the evaluator may cite only holdout, brief or docs paths. To pass grounding it has to cite a
  scenario file and quote it, which puts scenario text into the flow that writes fix stories.
- Resolution: give holdout symptoms their own schema without a location, exclude it from grounding, and
  let `holdoutFindings()` build the blocking findings from the scores.

### SEAM-44. Nothing records which files an amendment covers

- Severity: minor
- Documents: state §4 (answer to S7: `spec_locked` is false only while a live run's amendment names the
  file); hooks §2.11.3 (`artifact_frozen`: "no open amendment names it"); pipeline §2.6 and §2.9.4
  (`feature` and `spec` runs "hold an amendment").
- What breaks: the survey cannot know in advance which contracts a feature will touch, so approved
  files stay locked, or every amendment unlocks everything.
- Resolution: after the delta step, a `vsdd` command writes the amendment's file set into the run
  record from the delta data (`contracts_changed` and the related ids). The alternative is to define an
  amendment as covering the whole specification set.

---

## Checked and found sound

- The event log's location (`.factory/local/events/<date>.jsonl`), its fields, its one writer and the
  `VSDD_EVENTS_DIR` override agree across hooks §2.12, state §2.3 and packaging H3, as does the
  observability collector reading it.
- Python 3.11 as the floor, the standard library only, and the launcher paths (`py/launch.sh`,
  `py/hook.py`, `py/cli.py`, `bin/vsdd`) agree. Hooks' change to I3 (a mode argument, no `exec`) is
  stated and fits.
- Pre-1.0 and inert detection (`.factory/factory.toml`, `.factory/STATE.md`) agree in hooks §2.8,
  state §2.3 and packaging H4/S1, and packaging's cutover safety property is met.
- The state directory sits at `<root>/.factory` on branch `factory-artifacts`, as one instance, which
  answers hooks S9. Templates live at `<plugin>/templates/` and the research cache at
  `.factory/local/cache/research/`.
- The split of briefing ownership holds: state writes the text (1,200 characters), hooks deliver it
  (with a 2,000-character ceiling) on every `SessionStart` source, and `vsdd status` calls the same
  function.
- Worktree and branch conventions agree: `.worktrees/<story-id>/`, `.worktrees/_integration/`,
  `story/{id}`, `spike/{name}` and `.worktrees/spike-<name>/`.
- The step-record identity (run, workflow, subject, name, attempt), the `store_step` and
  `settle_decision` functions, the evidence path format and the decision statuses agree across pipeline
  S3 to S5, state §2.8 and hooks S7. The exception is the lookup semantics in SEAM-05.
- The MCP server keys, the `userConfig` secret keys, the two spellings of each tool name and
  `network.hosts` agree between agents-skills §2.10 and packaging §2.4.
- The configuration keys for commands, branches, delivery, convergence and holdout thresholds, and the
  default spike limits (2, 6 and 20 spikes, 20 minutes, 14 days' retention), agree between pipeline S1
  and state §2.12. The exception is the limit names in SEAM-36.
- The platform floor of 2.1.331 (`engines`) and the decision to drop the `PostCompact` and
  `WorktreeCreate` hooks are consistent across packaging and hooks.
