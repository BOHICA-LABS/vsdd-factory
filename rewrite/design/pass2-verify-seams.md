# Second pass: verification of the seam fixes

Reviewer task: check that the resolution stated in `rewrite/BLUEPRINT.md` section 8 for each of the 44 findings
in `rewrite/design/review-seams.md` was carried out in the blueprint and in every design document where the
problem appeared. Decisions D1 to D30 are treated as fixed. Written 2026-11-23 against the files on disk.

## Verdict

The first pass did most of the structural work. The tool now has one owner. The clerk and the scenario reviewer
exist in every roster. Workflows use roster names. Red evidence is looked up per gate. The spike skill is gone.
Multi-repository work follows the hub model. Configuration cannot be written by a tool call. Twenty of the 44
fixes are present and consistent everywhere I looked. The other 24 are not finished, and they fall into two
groups. Seven are real gaps:
- The scaffold story and any story that adds a dependency cannot write the files they need (blocker).
- Adding the scenario reviewer created new paths that leak hidden-scenario text to walled agents and to the
  main session.
- The `step record` fallback cannot be run by the nine agents that have no shell.
- Named spikes cannot get a ledger entry that the hook recognises.
- A brief opened by `discover` in assess mode starts work outside any cycle.
- The route table never starts a queued change request in a project that has no approved brief. That includes
  every project that went `ingest` then `feature`, which is the brownfield path the user asked about.
- Findings built from hidden-scenario scores have no defined shape, and as written they can never block.

The other 17 are places where the blueprint and the amendment list at the top of a design document say the
new thing, but the body of that document still says the old one. Each amendment list says it overrides such
passages. Builders work from the bodies, so each one is worth a short edit. Checked: 44. Verified: 20.

---

## Blocker

### P2SEAM-01. Scaffold, harness and dependency changes have no write scope (SEAM-18 partial)

- Severity: blocker
- Original finding: SEAM-18 (every change to the product tree is a story; readiness becomes `chore` stories).
- Locations: `BLUEPRINT.md` 4.3 row 13 (`devops` may write `ci`), 4.6 (`chore` = scaffold, CI, toolchain, built
  by `devops`; `harness` built by `verifier`); `state.md` 2.12 `[paths]` (`source = ["src/**",
  "crates/*/src/**"]`, `tests = [...]`, `ci = [".github/**"]`); `agents-skills.md` 2.9 write scopes
  (implementer "`source`, including build manifests"; devops "`ci` and repository configuration files";
  verifier "`tests` (harness directories)"); `hooks.md` 2.7 (`write` is "the complete list of classes the
  caller may write"), 2.11.2 `write-scopes` check 2; `pipeline.md` 2.9.5 readiness row.
- Problem: under the stated globs and write rule, a path that sits in none of `source`, `tests`, `docs` and
  `ci` can be written by no builder. That covers `Cargo.toml`, `package.json`, lock files, `pyproject.toml`,
  `rust-toolchain.toml`, `.gitignore` and similar files. It also covers `fuzz/**`, `benches/**`, and the
  proof harnesses that live under `src/`. The scaffold story must create `src/**`, `tests/**` and the
  manifest, but it is given to `devops`, which may write only `.github/**`. The implementer's "including
  build manifests" grant has no glob behind it. There is also a second gap. The scaffold is the point at
  which `[commands]` and `[paths]` first become knowable in an empty repository. `factory.toml` is
  person-only (SEAM-08), and nothing in the plan stops for a person to fill those values in after the
  scaffold merges.
- What breaks: the first wave of every greenfield project. The plan gate requires the scaffold story, and
  `devops` is blocked on its first write. Any story that adds a dependency is also blocked, so the D15
  "dependency change needs a `merge` decision" path is never reached. Harness stories in `verify` cannot
  create fuzz targets or benchmarks.
- Proposed resolution:
  - Add `[paths] manifests` and `[paths] config` (detected by `vsdd init`: manifests, lock files, toolchain and
    repository configuration files) and `[paths] harness` (`fuzz/**`, `benches/**`, proof directories).
  - Grant the implementer `source` and `manifests`, the verifier `tests` and `harness`, and `devops` `ci`,
    `config` and `manifests`.
  - In a `chore` story whose method is `repo-setup`, also grant `devops` `source:create` and `tests:create`.
  - When `vsdd init` finds an empty repository, have it leave the commands empty and record that the scaffold
    story will provide them. When that story is delivered, open an `environment` decision whose packet holds
    the exact `[commands]` and `[paths]` lines for the person to add. Make later stories in the wave depend on
    the scaffold story.
  - Add a hooks case: `devops` creates `Cargo.toml` and `src/lib.rs` in a `repo-setup` chore story (allowed);
    the implementer adds a dependency to `Cargo.toml` (allowed).

---

## Major

### P2SEAM-02. Scenario-review findings leak hidden-scenario text (SEAM-12 regressed)

- Severity: major
- Original findings: SEAM-12 (fix: a `scenario-reviewer` agent that reads `holdout`), interacting with
  SEAM-13 (claims carry `location`) and SEAM-10 (walled reads through the plugin's own command).
- Locations: `pipeline.md` 2.9.3 review row (adversary, scenario-reviewer and ui-reviewer in one `converge()`
  at the `spec` perimeter; "At the cap, open findings go to the packet"), 2.10.2 (`location.quote`), 2.10.8
  Claim row (`key`, `location`, `resolution`, `commit`); `state.md` 2.5 ("Findings still open when a loop
  ends are copied into the decision packet"; "the closure record keeps the dismissed and waived findings in
  full"), 2.4 class table (`decisions` and `control` are not walled), 2.9 briefing ("Read
  .factory/cycles/<cycle>/decisions/<id>.md"); `agents-skills.md` 2.9 read walls (builders and the main
  session are denied only `holdout`); `hooks.md` 2.7 `[reveals]` (`"review show" = ["runs"]`).
- Problem: a scenario reviewer's finding cites a hidden-scenario file and quotes it, as grounding requires.
  That text then travels to four places outside the `holdout` wall:
  1. Claims for the next pass go to every lens in the pass, so the adversary and the ui-reviewer receive
     `location.quote` taken from `HS-NNN.md`.
  2. The fix list goes to the planner named in `owner`. Nothing says a list holding scenario-located findings
     is split before the architect or ux-designer sees it.
  3. Open findings at the cap are copied into the `spec` decision packet, which builders and the main session
     can read. The briefing tells the main session to read it.
  4. Waived and dismissed findings are kept "in full" in the closure record (class `control`, readable by
     builders), and `vsdd review show spec` prints them. The hook treats that command as revealing only
     `runs`, which the main session is not denied.
- What breaks: wall W1 (only `product-owner`, `scenario-reviewer` and `holdout-evaluator` read hidden
  scenarios) and decision D6 (the main session is walled from scenarios). Builders can then build to the
  hidden scenarios, which is the reason the wall exists.
- Proposed resolution:
  - Run the scenario review as its own convergence loop at the `spec` perimeter, with its own claims, and send
    its findings only to `product-owner`.
  - Drop `quote` from the claim. Key, path, line range, resolution and commit are enough, because the next
    reviewer reads the file.
  - When writing packets and closure records, store any finding whose `location.path` is under
    `.factory/holdout/` under the holdout class, for example `holdout/reviews/<cycle>/<unit>.json`. Leave
    only its key and severity in the packet.
  - Make `vsdd review show <unit>` reveal `holdout` when the unit holds such findings, or redact them unless
    `--include-walled` is given.
  - Add a universal harness test: no prompt sent to an agent whose row denies `holdout` contains text from a
    finding located under `.factory/holdout/`.

### P2SEAM-03. The `step record` fallback cannot be run by the agents it exists for (SEAM-01 partial)

- Severity: major
- Original finding: SEAM-01 (fix: "`step record` is specified as the fallback of section 10").
- Locations: `state.md` 2.16, last row of the second table (`vsdd step record --json-file <path>`, "Any
  workflow step"); `hooks.md` 2.11.7, last paragraph; `pipeline.md` 7 risk R1; `BLUEPRINT.md` 10 item 1;
  `hooks.md` 2.7 `[callers]` (groups `read`, `measure`, `pipeline`, `spike`, `author`, `env`; no `step`);
  `BLUEPRINT.md` 4.3 (shell column) and 4.9 (the command is absent).
- Problem: if probe 1 shows the hook cannot see the whole `StructuredOutput` result, the plan B is that each
  agent runs `vsdd step record` as its last act. Nine of the 20 agents have no Bash tool: `product-owner`,
  `architect`, `ux-designer`, `story-writer`, `researcher`, `adversary`, `code-reviewer`,
  `security-reviewer` and `scenario-reviewer`. That is every pipeline reviewer. No `[callers]` group contains
  `step`, so `gate-guard` would also refuse the call from agents that do have a shell. The command is missing
  from the blueprint's command table.
- What breaks: if probe 1 fails, no review is stored. Verdicts, resume and grounding all depend on stored
  reviews, and the stated contingency gives no way to proceed.
- Proposed resolution: make the fallback independent of the agent's tools. Two options, both to be probed
  in B8 item 1:
  - Store the result from `PostToolUse` on `StructuredOutput`, where `tool_input` is complete.
  - Store it at `SubagentStop` by reading the final `StructuredOutput` call from the subagent transcript
    (`agent_transcript_path`).

  Keep `vsdd step record` only if a shell agent needs it. In that case add a `step` group to `[callers]`, add
  the command to blueprint 4.9, and say in R1 which agents cannot use it.

### P2SEAM-04. Named spikes cannot bind to their ledger entry, and the spike commands have no interface (SEAM-19, SEAM-20 partial)

- Severity: major
- Original findings: SEAM-19 (the named-spike step must be able to write in its own worktree) and SEAM-20
  (the ledger is written only through `vsdd`; one finding schema).
- Locations: `pipeline.md` 2.8 "Named spikes" (a clerk runs `vsdd spike open --name`, then a spike-runner
  step), 2.9.12, 2.11.1 `runSpike()` prompt; `hooks.md` 2.11.6, bound rows 1 and 4 and the containment
  bullet; `state.md` 2.5 ledger row (`agent`, `question_hash`, `name`), 2.16 first table (`vsdd spike open`,
  `record`, `close`, with no arguments).
- Problem:
  1. `spike-bounds` matches the entry by the runner's `agent_id`, which `spike open` writes "the first time it
     sees the runner". For a named spike the entry is opened by a clerk, so the runner's id is not in it, and
     the runner's working directory is a platform worktree, not `.worktrees/spike-<name>/`. Neither key
     matches, so every call except `spike open` is blocked. What a second `spike open --name` does to an
     existing open entry is not specified, nor whether it counts against `max_open`.
  2. A named spike advances over many steps, but two rules are written for single-step spikes. Bound row 4
     refuses a stop until the entry has a verdict, and `spike open` returns any earlier finding with the same
     question hash and ends the spike. So either each step records a verdict and the next step's open
     returns it and stops, or every stop is refused once and logged as `spike.stop_unrecorded`. Row 4 also
     matches by worktree while row 1 matches by agent id.
  3. The only statement of the arguments of `spike open` and `spike record` is the skeleton prompt in 2.11.1.
     It passes the question and the finding on standard input (`--question-file -`, `--json-file -`).
     `hooks.md` 2.11.10 says tool commands never take standard input, so the call falls outside `auto-allow`
     and prompts. The skeleton also passes `--asker` as a step label (`wf/name/subject`), not the agent type
     that the wall inheritance reads.
- What breaks: `/vsdd-factory:spike {open: …}` cannot advance a named spike, which is the user's existing
  convention and a stated requirement. Every quick spike asks for permission in an unattended run.
- Proposed resolution:
  - Specify the spike commands in `state.md` 2.16:
    - `spike open --question <tmp file> --asker-type <type> --run <id> --ref <ref> [--name <n>]`
    - `spike record --id <id> --finding <tmp file> [--increment]`
    - `spike close --name <n>`
  - State the binding rule. The hook records the runner's `agent_id` on its first `vsdd spike open`, and the
    tool attaches that id to the open entry it returns. For an existing open named entry, the runner's open
    attaches without deduplicating and without counting toward `max_open`.
  - Let a named step end with `spike record --increment`, which updates `.spike/STATE.md` without closing the
    entry. Have bound row 4 accept an increment record for a named entry.
  - Match by agent id in every row.

### P2SEAM-05. A brief opened by `discover` in assess mode starts a product cycle that is never opened (SEAM-21 partial)

- Severity: major
- Original finding: SEAM-21 (decisions need a cycle; fix: `cycles/_standing/`, and the brief checkpoint and the
  feature survey open a cycle).
- Locations: `BLUEPRINT.md` 4.11 `brief` row ("The brief skill ends, or `discover` in assess mode ends"), 4.9
  (`checkpoint brief` and the survey open cycles); `pipeline.md` 2.6 `brief` row, 2.9.9 assess paragraph
  ("a checkpoint that ... opens decision `brief`"); `state.md` 2.3 layout (`cycles/_standing/` holds
  `discover`), 2.7 "Who updates it" table, 2.8 cycle record row (opened only by `vsdd checkpoint brief` or
  `vsdd survey feature`).
- Problem: `discover` runs in `_standing`, and its checkpoint opens decision `brief`. No step opens the cycle
  that the approved brief then starts.
- What breaks: after the person approves, `spec`, `plan`, `wave`, `verify` and `release` all run with no
  cycle. They have no `cycles/<cycle>/` for closures, decisions, `demo/<cycle>/` or
  `holdout/evaluations/<cycle>/`. The briefing says "No cycle is open". A `feature` request is not queued,
  because no cycle is active. The release checkpoint has nothing to close. This is the route D20 recommends
  for a market check.
- Proposed resolution: the `discover` assess checkpoint opens the cycle when it opens decision `brief`, using
  the same code as `vsdd checkpoint brief`. Add it to the cycle-opener lists in `state.md` 2.7 and 2.8 and in
  blueprint 4.9 and 4.11, and add a fixture test: assess mode, then `brief` approved, then a survey of `spec`
  finds an open cycle.

### P2SEAM-06. The queued-request route row is shadowed, so brownfield projects never start a queued request (SEAM-29 partial)

- Severity: major
- Original finding: SEAM-29 (queued change requests; fix: `requests/CR-NNN.md` and a route row).
- Locations: `pipeline.md` 2.4 route table (the rows "No approved brief and no change request in flight →
  human", "Only open decisions remain → human" and "No cycle is open and a change request is queued →
  `feature`"), 2.9.7 close row ("`next` is done, or `feature` when a change request is queued"); `state.md`
  2.5 change-request row ("When the cycle closes the route starts `feature` with the oldest").
- Problem: the table is evaluated top to bottom. A project built through `ingest` then `feature` never has an
  approved brief. Whenever no request is in flight, the row "No approved brief and no change request in
  flight" matches first and returns a human instruction, so the queued-request row is never reached. The
  rows for `spec`, `plan`, `wave`, `verify` and `release` are not limited to an open cycle. After a release,
  "All waves closed and `verify` has not converged on the current commit" can match, because the release
  commit differs from the verified one, which contradicts the release checkpoint's stated `next`.
- What breaks: in every brownfield project the request queued during a cycle stays `queued`. The person is told
  to type `feature` and enters the request again. The old `CR-NNN` later starts a second time. The driver stops
  where it should continue. This is the brownfield path the user asked about.
- Proposed resolution: place "No cycle is open and a change request is queued" directly below the lock and
  multi-repository rows. Qualify the `spec` to `release` rows with "a cycle is open". Write the predicates in
  `routes.toml` with fixtures:
  - brownfield, cycle closed, one queued request: `feature` with that request;
  - greenfield after release, nothing queued: done;
  - a standing decision open and a request queued: `feature`.

### P2SEAM-07. Findings built from hidden-scenario scores cannot block as written (SEAM-43 partial)

- Severity: major
- Original finding: SEAM-43 (fix: holdout symptoms have their own shape with no location; `holdoutFindings()`
  builds the blocking findings from the scores).
- Locations: `pipeline.md` 2.9.2 ("one high finding per must-pass scenario under its floor, and one aggregate
  high finding"), 2.10.2 (`location` with `path` required; `spec_ref`, `repro` and `scope` required), 2.10.8
  Holdout data row, 2.7 blocking rule 2 ("A severe finding with no anchor is advisory"), 2.11.1 `judge()`
  (`hard` requires `spec_ref || repro` and `location.path`); `hooks.md` 2.11.7 step 3.
- Problem: the fix removes the location from symptoms but never says what the findings built from scores
  contain: their key, `location`, `spec_ref` or `repro`, and `scope`. `judge()` makes a severe finding
  advisory when it has no `location.path` or no anchor. The aggregate mean or spread finding has no natural
  file at all. If a builder fills `location` with the scenario path, it flows into the next round's claims
  to the adversary and into fix-story text.
- What breaks: a must-pass scenario below 0.6, or a mean below 0.85, is advisory, and the wave and release
  gates pass with the hidden-scenario check silently switched off.
- Proposed resolution: specify the derived finding as follows.
  - Key `holdout:<HS-id>`, and `holdout:<unit>:mean` or `holdout:<unit>:spread` for the aggregates.
  - Category `holdout`; scope `integration` at a wave and `system` at release.
  - `spec_ref` set to the contracts the scenario cites, resolved by the tool. `repro` set to
    `scenario <id>, seed <seed>`.
  - No location. In `judge()` and its Python copy, exempt category `holdout` from the location condition and
    from grounding.
  - Claims for these findings carry only the key.

  Add conformance fixtures for both thresholds.

---

## Minor

### P2SEAM-08. Most stage rows name no method, and some steps still have none to name (SEAM-04 partial)

- Severity: minor
- Original finding: SEAM-04 (every step names a method; add the missing methods).
- Locations: `pipeline.md` 2.9.1 to 2.9.11 stage tables, which are the content of `stages.toml` (`pipeline.md`
  4 I2, amendment 17: each stage carries `method`); `agents-skills.md` 2.8 method table; `packaging.md` 2.6
  universal test 6 and L-METHOD.
- Problem: most rows name an agent but no method. Examples are `spec` contracts, architecture, feasibility,
  assessments, scenarios and ux; `plan` outline and write; `feature` delta, story-draft and evolve; the story
  reviewers and fixer; the wave and verify reviewers; and `discover` briefs. Most have an obvious method in
  2.8. Four do not:
  - In `ingest` synthesize, the architect's "verification gap analysis" and the product-owner's "recovered
    contracts and scenario seeds" have no method.
  - In `maintain`, "pattern consistency" goes to the code-reviewer, whose only method is `review-diff`, and a
    sweep has no diff. "Docs drift" goes to the technical-writer with no method.
  - "Scenario freshness" goes to the holdout-evaluator alone. The old sweep (`maintenance-sweep` Sweep 4) also
    judged which scenarios are stale against intended behaviour, and the evaluator is walled from the
    contracts that judgement needs.
  - The test-writer fixes `coverage` findings with no method named.
- What breaks: universal test 6 fails, or builders invent methods per workflow, so the stage registry and the
  method files drift.
- Proposed resolution: add a method column to every 2.9 table. Add the missing methods, for example architect
  `verification-assessment`, product-owner `recover-contracts`, code-reviewer `review-patterns` and
  technical-writer `docs-drift`, and say which test-writer method fixes coverage findings. Split scenario
  freshness: the holdout-evaluator re-runs the scenarios, and the `scenario-reviewer` (method
  `review-scenarios`) judges staleness and coverage against the contracts.

### P2SEAM-09. The agents-skills result table still describes the old contracts (SEAM-09, SEAM-14, SEAM-20, SEAM-38 partial)

- Severity: minor
- Original findings: SEAM-09 (devops no longer merges), SEAM-14 (field names follow the schemas), SEAM-20
  (the spike runner returns the spike-finding schema), SEAM-38 (no `wave-plan`).
- Location: `agents-skills.md` 2.6 table. The paragraph above it says the schema names win and that pipeline
  reviewers get no report path, but the rows were not rewritten.
- Problem: the rows contradict the amendments:
  - `devops` still performs "`git` and `gh` operations" and returns `worktree_path`, `pr_number`, `ci_state`
    and `merge_sha`.
  - `story-writer` writes a "wave plan" and returns `waves`.
  - `spike-runner` writes "one ledger entry" and returns verdicts "confirmed, refuted or inconclusive", plus
    `result`, `worktree` and `branch`.
  - `implementer` is given "the review report paths".
  - `adversary` returns `counts`, `report_path`, `not_reviewed` and `out_of_scope`.
  - `code-reviewer`, `security-reviewer`, `holdout-evaluator` and `ui-reviewer` are each given a report path
    and write "one review report" or "one evaluation report". The holdout-evaluator returns `score` and
    `gaps`, where the schema has `satisfaction` and `symptoms`.
- What breaks: package A6 writes the Result and Inputs sections from this table. An agent file built from it
  tells the holdout evaluator to write a file its row forbids (`write = []`) and to return fields the schema
  rejects.
- Proposed resolution: rewrite each row in the schema's field names and remove the report-path and
  merge-related entries.

### P2SEAM-10. The severity text exists in three wordings, and `owner` cannot name the demo recorder (SEAM-14 partial)

- Severity: minor
- Original finding: SEAM-14.
- Locations: `pipeline.md` 2.7 severity table; `BLUEPRINT.md` 4.10 last paragraph (a shortened paraphrase
  that it says is "copied word for word"); `agents-skills.md` 2.7 adversary example, Rules bullet "Severity,
  exactly as the pipeline defines it" (a third wording); `packaging.md` 2.6 L-AGENT ("the severity table ...
  byte for byte"); `pipeline.md` 2.10.2 `owner` enum.
- Problem: lint compares the reviewer files byte for byte against one text, but the three sources differ
  ("an exploitable security hole" against "an exploitable hole"; "a real defect with a concrete consequence
  that no contract covers" against "a real defect no contract covers"). The `owner` enum also has no
  `demo-recorder`, so a release-perimeter finding against the final demo has no owner.
- What breaks: an adversary file copied from the example fails L-AGENT. Demo defects are misrouted to the
  implementer.
- Proposed resolution: make the `pipeline.md` 2.7 table the single text. Reproduce it exactly in blueprint 4.10
  and in the example, as the table, not inline prose. Add `demo-recorder` to the `owner` enum.

### P2SEAM-11. The wall examples and the agents-skills translation paragraph contradict the amended tables (SEAM-35 partial)

- Severity: minor
- Original finding: SEAM-35 (class names; deny `runs` to reviewers).
- Locations: `hooks.md` 2.7 `walls.toml` example rows; `hooks.md` 2.7 key table (`vsdd`: "Absent means `read`
  only") and the paragraph after `[reveals]` ("A builder row grants `read` and `measure`"); `hooks.md`
  amendment 4; `agents-skills.md` 2.9 paragraph "In the hooks draft's format".
- Problem: the example rows do not match amendment 4 or the surrounding prose:
  - `implementer`, `test-writer` and `adversary` do not deny `runs` (wall W3).
  - `adversary` does not deny `outside`.
  - The builder rows have no `vsdd` key, which by the key table means read only, so `vsdd gate red` and
    `vsdd gate green` would be refused.
  - `[main]` has no `vsdd` key, although the prose says it grants every group.

  The agents-skills paragraph still describes `zones.toml`, `roles.toml`, `holdout-reports` and the flags
  `write_source_before_red` and `amend_tests`. It says only `product-owner` and `holdout-evaluator` need
  `read_allow`, which leaves out `scenario-reviewer`, and that "create only" cannot be expressed. All of that
  is superseded by `classes.toml`, `walls.toml` and `:create`.
- What breaks: package C2 built from the example breaks the TDD loop and wall W3.
- Proposed resolution: correct the example rows (`runs` in every row, `outside` for agents without a shell,
  `vsdd = ["read", "measure"]` for builders, an explicit `[main]` grant). Delete the agents-skills paragraph
  or replace it with a pointer to `hooks.md` 2.7.

### P2SEAM-12. `[reveals]` keys on kind names that nothing defines, and the second line of defence is not in the tool's design (SEAM-10 partial)

- Severity: minor
- Original finding: SEAM-10.
- Locations: `hooks.md` 2.7 `[reveals]` (`"list holdout"`, `"list story"`, `"list contract"`) and the paragraph
  after it (`--include-walled`); `state.md` 2.6 ("contracts, properties, stories, scenarios, subsystems,
  research"), 2.16 (`vsdd list <kind>`, no kind list, no `--include-walled`); `BLUEPRINT.md` 4.9.
- Problem: the kind vocabulary of `vsdd list` is defined nowhere. State calls them "scenarios", and the reveals
  table says `holdout`. An unlisted kind has no default. The `--include-walled` filter appears only in the
  hooks design, not in the command table of the design that builds the command.
- What breaks: if the S1 builder names the kind `scenario`, `vsdd list scenario` passes the wall for every
  builder (W1).
- Proposed resolution:
  - Define the `list` kinds as the artifact kinds in `artifacts.toml`, and generate `[reveals]` from each kind's
    path class.
  - Treat an unknown `list`, `refs` or `coverage` argument as revealing every walled class.
  - Add `--include-walled` to `state.md` 2.16 and blueprint 4.9.

### P2SEAM-13. Documented decision choices do not fit the decision grammar (SEAM-37 partial)

- Severity: minor
- Original finding: SEAM-37 (fix: grammar `<id> <choice> [key=value ...] [-- note]`).
- Locations: `pipeline.md` 2.6 (`revise "<guidance>"`, `hold "<guidance>"`, `waive <keys> "<reason>"`,
  `waive-coverage <items> "<reason>"`, `direct "<guidance>"`, `not-a-gap "<reason>"`, `open-spike <name>`);
  `BLUEPRINT.md` 4.11 (`waive <keys>`, `waive-coverage <items>`, `open-spike <name>`); `hooks.md` 2.11.5
  step 2 ("or finding keys after `waive`" listed as key=value pairs); `state.md` 2.8 choice table.
- Problem: the grammar has neither positional values nor quoted free text. The documented forms pass both.
  Finding keys (`<category>:<path>:<symbol>`) are not `key=value`.
- What breaks: a parser built to the grammar either refuses `waive <key> "<reason>"` or puts the keys into the
  note. A waiver then records no keys and the finding stays blocking. A person who types the documented form
  is refused.
- Proposed resolution: use named forms everywhere: `waive key=<k> [key=<k> ...] -- <reason>`,
  `waive-coverage item=<i> ... -- <reason>`, `open-spike name=<n>`, `revise -- <guidance>`. Update
  `pipeline.md` 2.6, blueprint 4.11, `hooks.md` 2.11.5 and the `decide` skill's `argument-hint`. Add parser
  fixtures for each kind.

### P2SEAM-14. The deletion commands have no caller in the lifecycle that needs them (SEAM-30 partial)

- Severity: minor
- Original finding: SEAM-30 (fix: `vsdd defer resolve`, `lesson retire`, `research retire`).
- Locations: `BLUEPRINT.md` 4.9 (these are main-session commands); `hooks.md` 2.7 `[callers]` (`author` group,
  granted only to the main session); `state.md` 2.5 (lessons "folded into `conventions.md` or
  `review-rules.md` by the architect and removed with `vsdd lesson retire`"; a superseded research report "is
  deleted"); `agents-skills.md` 2.12 (`defer` "Instructs the main session to write the entry"; `retro` and
  `research` fork the researcher); `pipeline.md` 2.9 (no step folds lessons).
- Problem: the commands exist, but the roles named for them, the architect and the researcher, have no shell.
  No workflow step folds lessons, and no skill or clerk step runs `lesson retire` or `research retire`. The
  `defer` row does not name `vsdd defer resolve`.
- What breaks: lessons fill the cap of 30 adopted lessons, and the retrospective is refused new ones. Superseded
  reports stay. Both become standing status problems that only a person who knows the command can clear.
- Proposed resolution: add the fold to a step. For example, in `release`, after `retro`, the architect folds
  adopted lessons and returns their slugs, and a clerk runs `vsdd lesson retire`. Have the researcher return
  `superseded` paths for a clerk to retire. Name `vsdd defer add/resolve` in the `defer` skill row.

### P2SEAM-15. The tool cannot tell which session's marker is its own (SEAM-22 partial)

- Severity: minor
- Original finding: SEAM-22 (fix: the survey requires a fresh hook marker).
- Locations: `hooks.md` 2.11.10 "Hook liveness"; `state.md` 2.11, second bullet, and amendment 12; `BLUEPRINT.md`
  4.13 "Hooks must be alive".
- Problem: markers are written per session under `local/hooks/sessions/<session id>.json`. A `vsdd` process
  started by the Bash tool does not know its own session id, and nothing says how it finds its marker. State
  2.11 supports two sessions in one project at once (a wave and a report-only sweep).
- What breaks:
  - In session B with hooks off, the survey sees session A's fresh marker and starts a run with the gates off.
  - The lock can record session A as the holder, so `lifecycle-log` releases B's lock when A ends, and a
    second run can take over a live one.
- Proposed resolution: have the hook pass the session to the call it allows. Either `PreToolUse` returns
  `updatedInput` adding `--session <id>` to a `vsdd` command, or the marker is keyed by a hash of the exact
  command and `vsdd` looks up its own. Require the survey to find that session's marker, and add the
  mechanism to the B8 probes.

### P2SEAM-16. One failure message still names a fix that cannot run (SEAM-25 partial)

- Severity: minor
- Original finding: SEAM-25 (messages name a form that works).
- Location: `hooks.md` 2.9 failure table, row "uv cannot run (cache not writable, unknown flag on an old uv)":
  "Exit 2, naming the exit code and `vsdd doctor`".
- Problem: `vsdd doctor` starts through the same `launch.sh cli` and `uv run`, so it fails the same way. The
  other rows name terminal commands that need nothing from the plugin.
- What breaks: the person following the message hits the same failure with no further hint.
- Proposed resolution: name `uv --version`, `uv cache dir` and the uv upgrade or reinstall commands, as the
  `uv_missing` row does.

### P2SEAM-17. After a fix, no result carries the new diff patch (SEAM-31 partial)

- Severity: minor
- Original finding: SEAM-31 (fix: the tool writes the diff patch at the survey and after each fix).
- Locations: `state.md` 2.16 (`vsdd gate green` writes `diff.<sha12>.patch`); `pipeline.md` 2.10.6 gate result
  (no patch field), 2.10.8 Story unit (`diff_patch` from the survey) and Delivery data (no patch field), 2.11.2
  (`review(d.reviewed_sha, 2, d.changed_files)` after a repair).
- Problem: the patch is written after a fix, but no schema returns its path to the script. The scoped
  re-review after a repair needs a patch from `reviewed_sha` to head, which nothing in the `story` workflow
  writes. `vsdd review diff` is never called there. Reviewers have no shell.
- What breaks: pass 2 reviewers and post-repair reviewers receive the survey's stale patch, or none.
- Proposed resolution: add `diff_patch` to the gate result of `green`, and to delivery data for the
  `reviewed_sha..head` range. The script hands the latest one to each reviewer.

### P2SEAM-18. `next` is still a string in the status record, and the route rule still appears in two languages (SEAM-40, SEAM-16 partial)

- Severity: minor
- Original findings: SEAM-40 (`next` is one object everywhere; the route table is data) and SEAM-16 (the
  conformance layer).
- Locations: `state.md` 2.7 example (`"next": "/vsdd-factory:wave"`) and field table ("The exact next command,
  or `human: <what to do>` ... 200 characters"); `pipeline.md` 5.4 P4 ("judge() / converge() and route rule"
  in JavaScript and in the tool), 7 R2 ("The convergence and route rules exist twice"), 2.5 ("a convergence
  verdict, a route ... the tool recomputes it"); against `packaging.md` 2.6 layer 2b and `pipeline.md`
  amendment 12 (only the tool computes the route).
- What breaks: the status-record schema and the briefing renderer are built to a string. A prelude builder may
  add a JavaScript route function that nothing tests against the tool.
- Proposed resolution: show `{kind, command, args}` in the example and field table, with the 200-character
  limit applied to the rendered text. Remove "route rule" from P4, R2 and 2.5.

### P2SEAM-19. Amendment edits are not required to return files to draft (SEAM-44 partial)

- Severity: minor
- Original finding: SEAM-44 (fix: an amendment covers the whole set; every edited file returns to `draft`).
- Locations: `state.md` 2.4 `frozen_when` row ("the edit must set the status back to `draft`"); `hooks.md` 2.11.3
  check table (`artifact_frozen`: "no open amendment names it"; `code_only_value` only stops adding an approved
  value) and 5 S7 (`spec_locked` "no open amendment names it").
- Problem: no hook check enforces the reset to draft. Keeping `status: approved` while changing the text passes
  every check. Two places still describe a per-file lock ("names it").
- What breaks: an amended contract keeps `approved` on text no person approved. The packet's list of changed
  files, which relies on drafts, is incomplete.
- Proposed resolution: add check `amendment_status`. When an amendment allows an edit to an approved file, the
  new content must set `status: draft`. Reword both places to the whole-set rule of `state.md` amendment 15.

### P2SEAM-20. Interface tables still state superseded needs (SEAM-17, SEAM-28 partial)

- Severity: minor
- Original findings: SEAM-17 (hooks meet the state's needs H-2 to H-5 and adopt the five conditions) and SEAM-28
  (one story-kind list).
- Locations: `hooks.md` 5 S7 (`spike_entry(project, worktree)` without `agent_id`; no `unit_tree`, `lock_seen`
  or `lock_release`); `hooks.md` 2.8 condition table (four conditions, with "partial" defined by the status
  record and the schema version, unlike `state.md` 2.3); `pipeline.md` 5.3 H4 G1 ("kinds `facade` and `docs`
  excepted").
- Problem: these are the tables that the hook packages H1 to H9 build against. They disagree with the state
  design's S7 (the agent-id signature and the three added functions), with its `detect()` conditions (partial
  means a failed mount test; `unmounted` exists), and with blueprint 4.6 (`chore` and `harness` are also
  exempt).
- What breaks: an H package built to these tables calls the wrong signatures or exempts the wrong kinds.
- Proposed resolution: update `hooks.md` 5 S7 to the state design's list. Replace the 2.8 table with the five
  conditions of `state.md` 2.3. Correct G1 to `facade`, `docs`, `chore` and `harness`.

---

## Checked and found sound

Verified: the fix is present and consistent in the blueprint and in every design document where the problem
appeared.

- SEAM-02: `clerk` exists in all five places: the roster (4.3; agents-skills 2.4 and 2.5), the four-tier
  `roster.toml`, P-8 and L-AGENT (low effort for the clerk tier only), and the `walls.toml` row (`tool_only`,
  `write = []`, `holdout`, `reviews`, `runs`, `outside`).
- SEAM-03: no old agent name survives in a pipeline stage table. Blueprint 4.4 maps each one. `ROLE` is
  generated, and universal test 3 checks agent types.
- SEAM-05: per-gate evidence lookup is the same in `state.md` 2.8 and S7, `hooks.md` 2.11.4 (with the two
  cases) and blueprint 4.13.
- SEAM-06: there is no spike skill anywhere. `model_visible_skills = 1`, `/vsdd-factory:spike` is the workflow,
  and the skill roster has 13 entries.
- SEAM-07: the hub model is in `pipeline.md` 2.9.7 and 2.9.11, `hooks.md` 2.5 and 2.8, and `state.md` 2.14. Each
  `[[repo]]` entry carries its own `branches`, `commands` and `paths`.
- SEAM-08: `config` has `write = "none"` in `state.md` 2.4 and `hooks.md` 2.7. `config set` is person-only. The
  `setup` skill only shows values. Gate-bearing sections are fingerprinted and announced.
- SEAM-11: the `PreToolUse` matcher names the Playwright navigation and upload tools in both spellings, `WebFetch`
  and the research tools. A `file:` address counts as a read, and the bundled server runs with file access off.
- SEAM-13: claims are `key`, `location`, `resolution`, `commit` in 2.7, 2.10.8, agents-skills 2.9 and the
  adversary example. Universal test 8 checks reviewer prompts. The quote inside `location` is the subject of
  P2SEAM-02, not a builder-text leak.
- SEAM-15: `roster.toml` is the single source. Tier and role tables are generated. The spike runner sits on one
  tier (planner, D5).
- SEAM-23: the `decide` skill (person-only), the `UserPromptExpansion` wiring and `vsdd checkpoint brief`
  (state 2.16) all agree.
- SEAM-24: the `drive` skill, `vsdd driver on/off`, `local/driver.json`, and the briefing's continue
  instruction (state 2.9) agree.
- SEAM-26: the exempt subcommands, `stale` without `--update`, `status [--fix] [--json] [--wave]` and exit 3 for
  the canary only are identical in packaging 2.10, state 2.16, hooks 2.13 and blueprint 4.9.
- SEAM-27: the `feature` fast route has a clerk step for `vsdd gate plan --story`, with two fix rounds.
- SEAM-32: `vsdd export pdf` and `vsdd render diagram` exist in packaging 2.10, `doctor` checks Node, and the
  skills call them.
- SEAM-33: `location.path` is relative to `tree`. The survey returns `tree` per unit, and grounding resolves it
  through `unit_tree` with three lines of drift.
- SEAM-34: the tool is `vsdd` everywhere, the markers are `stamped:prelude`, the source is
  `workflow-lib/prelude.js`, and L-XREF checks every `vsdd <subcommand>`.
- SEAM-36: the prelude-name to `factory.toml` key mapping is identical in blueprint 4.12 and state 2.12. The
  phantom keys are gone, `max_open` is enforced, and universal test 9 exists.
- SEAM-39: the packaging layout carries `methods/`, `schemas/`, `pipeline/`, `assets/` and the Python
  sub-packages. L-METHOD and L-TEMPLATE exist.
- SEAM-41: `VSDD_TEST_TIER` becomes a survey tier override. The scheduled-run recipe is in
  `docs/running-the-pipeline.md`.
- SEAM-42: ancestor searches are allowed for rows whose denied classes all lie under `.factory/`, once the
  probe (blueprint 10 item 14) confirms the exclusion. Walled product classes stay blocked.

Count: 44 findings checked, 20 verified, 24 reported above as P2SEAM-01 to P2SEAM-20. The 24 are 23 partial
and 1 regressed (SEAM-12). Several findings above cover more than one original.
