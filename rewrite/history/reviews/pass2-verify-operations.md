# Second pass: verification of the fixes for the operations findings (OPS-01 to OPS-31)

Reviewer: second-pass verifier, operations lens. Read: `rewrite/STATE.md`, `rewrite/BLUEPRINT.md` (all of it),
`rewrite/design/review-operations.md`, and the five design documents (amendment lists and every body section
each finding touched). Decisions D1 to D30 are treated as adopted. Written 2026-11-23.

## Verdict

The first pass did most of the work it claims for this report. All four blockers have their fix in the
blueprint and in the owning designs: long gates and delivery run as detached jobs (OPS-01), the decision digest
leaves out `status` and settling writes the decision first (OPS-02), `decide` and `drive` exist with a terminal
shim (OPS-03), and references are checked for existence only at gates and approval (OPS-04). None of them is
still a blocker. Of the 31 findings, 12 are verified. 19 are partial: the fix is in the blueprint, but a design
document still says the old thing, or the stated fix does not solve all of the original problem. None is
missing outright. Four of the partial fixes are major. (1) The run lock and the hooks-alive check depend on a
session marker that `vsdd` cannot match to its own session when two sessions are open (OPS-07, OPS-14). (2) Decision
D4 is adopted, but the designs describe two different allow mechanisms, and in the hook-based one `setup` cannot
switch the allow-list on, because no tool call may write `factory.toml` (OPS-12). (3) The default Rust globs put
`src/**/tests.rs` in both the `source` and `tests` classes, and the rules as written then block the test-writer from
its own tests after red (OPS-18). (4) The wave precondition "earlier waves closed" contradicts the route row and the
"queued, not blocking" rule that the OPS-05 fix relies on. The remaining 15 partial fixes are minor: wording left in
body sections, the pre-blueprint script skeletons, and small mechanisms the blueprint row promises but no
document specifies. One fix created a small new problem: `vsdd doctor --fix` is named as the only Python
installer, but the launcher cannot start it without the Python it is meant to install (P2OPS-16). Of the user's
two brownfield cases, P2OPS-03 hits an `ingest`-then-`feature` run on an existing Rust repository first.

Counts: 31 findings checked, 12 verified, 19 partial (4 major, 15 minor), 0 not fixed. One extra cross-cutting
finding, P2OPS-20, covers the pre-blueprint script skeletons. Four of the verified fixes (OPS-01, OPS-08, OPS-23,
OPS-26) are sound everywhere except those skeletons, which `pipeline.md` 2.11 explicitly marks as superseded.

## Findings

### P2OPS-01 — The session marker cannot tell `vsdd` which session called it (OPS-07, OPS-14)

- **Severity:** major
- **Locations:** `state.md` amendment 12 and 2.11, second bullet ("`vsdd` learns which session is calling from
  the marker the hook runner writes under `local/hooks/` just before it lets a `vsdd` call through");
  `hooks.md` amendment 8 and 2.11.10, "Hook liveness" (marker at `.factory/local/hooks/sessions/<session id>.json`;
  "The same marker is how the tool learns the session id for the run lock"); `BLUEPRINT.md` 4.13, "Liveness" and
  "Hooks must be alive"; `state.md` 2.16, `vsdd driver on` ("with the session id"); `BLUEPRINT.md` section 10
  (no probe for either assumption below).
- **Problem:** The original problem was that a `vsdd` process cannot learn its session. The fix has the hook
  write one marker file per session before each `vsdd` call. A `vsdd` process does not know its own session id, so
  when two sessions are open it has nothing to choose between their marker files with. Two sessions at once is
  the whole reason section 2.11 exists. The survey's hooks-alive test has the same flaw: "no fresh marker exists"
  is satisfied by another session's marker. Separately, liveness ("on any guarded tool call from the holder's
  session it refreshes `seen`") assumes that tool calls made inside workflow steps carry the main session's
  `session_id`. During a wave the main session makes no calls of its own, because it is waiting on the Workflow
  tool. Section 10 has no probe for that assumption.
- **What breaks:** The lock is recorded under the wrong session. The hook then refreshes `seen` for the wrong
  session, and on that session's `SessionEnd` it releases a live run's lock. After 20 minutes a second survey takes
  over the scope, and two implementers write in the same `.worktrees/<story>/`. That is the failure OPS-07
  described. A session whose hooks are off at the platform level (`disableAllHooks` in its own settings, or a
  plugin that failed to load) passes the survey's hooks-alive test whenever another session in the same project
  has made a `vsdd` call within the freshness window. That reopens the OPS-14 hole. `vsdd driver on` can write
  `driver.json` for the wrong session, so the briefing tells the wrong session to continue the driver.
- **Resolution:** Bind the marker to the call. On `PreToolUse` for a `vsdd` call, the runner rewrites the
  command through `updatedInput` to `VSDD_SESSION=<id> VSDD_HOOK_NONCE=<n> vsdd ...` and writes the nonce into
  that session's marker. `vsdd` accepts only a marker whose nonce matches its environment, and the survey
  refuses when none matches. Add to section 10: (a) `updatedInput` on a Bash call is honoured, inside workflow
  steps too; (b) the hook payload of a workflow step's tool call carries the main session's `session_id`.
  Fallback for (a): key the marker by the PID of the nearest Claude Code ancestor process, which the hook and the
  `vsdd` process can each find. Fallback for (b): the hook refreshes `seen` for any call whose `cwd` lies in a
  worktree the locked run owns. Record both in `state.md` 2.11, `hooks.md` 2.11.10 and blueprint 4.13.

### P2OPS-02 — Decision D4 has two incompatible mechanisms, and `setup` cannot switch on the hook one (OPS-12)

- **Severity:** major
- **Locations:** `BLUEPRINT.md` D4 ("the plugin approving its own routine tool calls ... A call is approved only
  when every other hook rule passes"; "only the default of one hook rule changes"), 4.5 `setup` row, section 10
  item 7 (static settings rules are the fallback); `hooks.md` 2.10 rule 17 ("Project list off by default"),
  2.11.10 `auto-allow` ("setup offers it once per project, and when the person accepts it permits ..."), 2.6 (the
  `[hooks]` table is changed only by a person editing the file); `state.md` 2.12 (`[hooks]`: "Reserved. No `vsdd`
  command changes it"; no tool call may write `factory.toml`); `packaging.md` amendment 12 ("`setup` offers to
  write, into the project's local settings, the permission allow rules of decision D4"); `agents-skills.md` 2.12
  `setup` row (setup runs `vsdd init` first and offers the allow-list afterwards). For the `-C` part:
  `hooks.md` 2.11.1 and 2.11.10 are the only places `vsdd -C <dir>` appears. It is absent from `state.md` 2.16 and
  `BLUEPRINT.md` 4.9, the command definitions, and `pipeline.md` 2.11.1 `relay()` still tells the clerk "Run exactly
  this command once in ${RUN.dir}".
- **Problem:** The user adopted D4 as a hook that approves routine calls. `hooks.md` builds that, defaulting
  off until `setup` turns it on. `packaging.md` builds a different thing: static `permissions.allow` rules in
  `.claude/settings.local.json`, which section 10 names as the fallback for when hook approval does not work
  inside agents. If the hook form is built, `setup` has no way to record the person's yes. The switch lives in
  `[hooks]` in `factory.toml`, no tool call may write that file, and `setup` runs `vsdd init` before it makes
  the offer. Two smaller parts of OPS-12 are also not carried through. First, `-C <dir>` is not defined on the
  tool, and the clerk prompt still invites `cd <dir> && vsdd ...`, which the clerk's `tool_only` rule refuses.
  Second, D4's command scope (the configured build and test commands, and five git verbs) leaves out the routine
  shell calls builders make: a filtered test run, `ls`, `mkdir`, toolchain helpers. In the default mode those
  still raise prompts.
- **What breaks:** Package K1 (setup) and package H6 (`auto-allow`) are built to different designs. In the hook
  form the allow-list stays off unless a person hand-edits `factory.toml`, so a six-story wave raises the dozens to
  hundreds of prompts that D4 was adopted to prevent, and scheduled runs cannot work. Clerk steps whose model
  prefixes `cd` are blocked, `relay()` retries once, and then the survey or checkpoint step fails.
- **Resolution:** Build D4 as the hook permit. `setup` asks before it runs `vsdd init`, and passes the answer as
  `vsdd init --auto-allow`, so the setting goes into the first `factory.toml` that `init` writes. `vsdd import`
  takes the same flag. `init` is already the one sanctioned writer of that file. Keep the static settings rules
  only as the fallback of section 10 item 7. Then align `packaging.md` amendment 12, the `agents-skills.md` setup
  row, `hooks.md` rule 17 and blueprint 4.5. Add `-C <dir>` as a global option in `state.md` 2.16 and blueprint
  4.9, and make `relay()` pass `vsdd -C ${RUN.dir} <command>`. Extend smoke probe 7 (`packaging.md` 2.6) to
  count prompts from builders' own shell calls. If those calls prompt, ask the user whether D4's scope covers a
  builder's shell call inside a story worktree that passes every other rule.

### P2OPS-03 — In Rust projects, `src/**/tests.rs` belongs to both `source` and `tests` (OPS-18)

- **Severity:** major
- **Locations:** `state.md` 2.12 `[paths]` defaults (`source = ["src/**", "crates/*/src/**"]`,
  `tests = [..., "**/tests.rs"]`), 2.4 class table ("Every other class is a directory no other class shares";
  the project-code classes come from configuration and are not covered by that sentence), `BLUEPRINT.md` 5.2 last
  bullet; `hooks.md` 2.11.4 `tests-first` ("For a path in the project's `source` class ... `source:before-red`
  (the test writer) is blocked once that record exists"), 2.11.2 `write-scopes` check 2; `pipeline.md` 2.9.1 (the
  red gate "does not count a test defined inside a source file").
- **Problem:** The OPS-18 fix moved Rust unit tests into `tests.rs` submodules and added `**/tests.rs` to the
  test globs. Every such file also matches the `source` glob, and no document says which class wins. Under the
  rules as written, `tests-first` treats the file as source. So does the red gate's test for "a test defined
  inside a source file", and so does `write-scopes`, which allows the write when any granted class contains the
  path.
- **What breaks:** This hits every Rust project the user runs, greenfield and brownfield alike. One reading
  gives the OPS-18 deadlock: the red gate counts no test in `tests.rs`, and after red the test-writer is blocked
  (`red_already_recorded`) from fixing its own tests in `tests.rs` when a review routes a test finding to it. The
  other reading loses protection: the implementer's `source` grant lets it edit the non-frozen tests in `tests.rs`
  that earlier stories wrote, which "the implementer cannot weaken any test" (`hooks.md` 2.11.4) rules out.
- **Resolution:** State one precedence rule in `state.md` 2.4 and `classes.toml`: a path that matches a `tests`
  glob is in `tests` even when a `source` glob also matches. Apply it in `write-scopes`, in `tests-first` and in
  the red gate. Add hook cases for `src/<m>/tests.rs`: the test-writer after red is allowed, the implementer is
  blocked, and the red gate counts the tests.

### P2OPS-04 — The wave precondition contradicts the route row and the "queued, not blocking" rule (OPS-05)

- **Severity:** major
- **Locations:** `pipeline.md` 2.9.2 survey row ("Preconditions: ... earlier waves closed") against 2.9.2
  "Ends" ("The next wave still starts if its stories do not depend on what is blocked"); `pipeline.md` 2.4 route
  table ("Every unit that could run waits on an open decision" above "A wave is open, or a later wave has runnable
  stories"); `pipeline.md` 2.6 "Exceptional stops" and `BLUEPRINT.md` 4.11, last paragraph ("the route continues
  with every unit that does not depend on it").
- **Problem:** The OPS-05 fix depends on the route knowing which units "could run". When a wave gate reaches
  its cap and opens a `finding` decision, the wave cannot close. The precondition then refuses every later wave,
  even one whose stories do not depend on what is blocked. No document says whether the route's notion of
  "could run" applies that same precondition.
- **What breaks:** If the route treats the later wave's stories as runnable, `next` is `wave`, the survey
  refuses, and the driver burns two runs before it stops. The briefing keeps naming `wave`, which the survey
  keeps refusing: the same disagreement between route and survey that produced OPS-05. If the route does apply
  the precondition, then 2.9.2 "Ends" and blueprint 4.11 promise something the build will not do: one open wave
  finding halts the whole build until a person answers.
- **Resolution:** Pick one rule and write it in all three places. Either (a) a wave whose gate waits on a
  decision blocks later waves: delete the "next wave still starts" sentence and add that exception to 4.11. Or
  (b) the precondition becomes "earlier waves closed, or waiting only on decisions that none of this wave's stories
  depend on". In either case, implement "could run" in the tool as the same function the survey's preconditions
  use, and add a scenario test: a wave gate reaches its cap with a `finding` decision, and the route returns
  `human` with that decision on the first try.

### P2OPS-05 — Settling still does all the promotion and a flush inside the gate-path hook, and the promised test is missing (OPS-02)

- **Severity:** minor
- **Locations:** `hooks.md` 2.11.5 steps 2 to 4 and 2.3 (the `gate` path's 4-second runner deadline; "a rule may
  start at most one `git` subprocess, with a 1.5 s timeout"); `state.md` 2.7 ("`vsdd decide`, called by the hook
  runner"), 2.3 (flush runs after `vsdd decide`, and it validates every changed file, then runs `git add` and
  `git commit`), 2.8 (promotion of every draft specification file happens inside `settle_decision`); the
  `BLUEPRINT.md` section 8 OPS-02 row ("a fixture test covers approve, survey, plan"), which no design document
  carries: not `packaging.md` 2.6, not `state.md` K-2, not build package S4.
- **Problem:** The original resolution asked the next survey to finish the promotion "so the hook's work stays
  small". As written, the hook writes the decision, promotes every draft specification file (`prism` has 278
  contracts) and then flushes, which means two or more git subprocesses, all within 4 seconds. Ordering the
  decision first makes an interruption safe, but the timeout still fires, and the regression test the blueprint
  promises is not in any test plan.
- **What breaks:** On a large specification the person who types `approve` sees a `runner_timeout` block, even
  though the decision was recorded. When they type it again they are refused with "already settled", and the
  model never receives the "next command" context, so the driver does not pick up. The hook also breaks its own
  rule budget. Without the fixture test, a later change to `digest()` can bring the stale-approval loop back
  unnoticed.
- **Resolution:** In the hook path, `settle_decision` writes only the decision record and the status record.
  The next survey, or the next `vsdd` command that flushes, does the promotion and the flush, which is what
  amendment 5 already allows for. Add the fixture test (approve `spec`, run the survey, the plan precondition is
  met) to package S4 and to `packaging.md` 2.6, layer 1.

### P2OPS-06 — After a decision the driver is off, so each routine stop costs two typed commands (OPS-03)

- **Severity:** minor
- **Locations:** `agents-skills.md` 2.12, the `drive` row (it "stops at a human decision ... and then runs
  `vsdd driver off`") and the `decide` row and skeleton (the skill only relays); `hooks.md` 2.11.5 step 4;
  `state.md` 2.9 (the continue line appears only while `driver.json` names the session); `pipeline.md` 2.4 ("three
  human actions after the brief conversation"); `BLUEPRINT.md` section 1 ("Each is one typed command. Everything
  else runs unattended").
- **Problem:** The driver turns itself off at every human stop, and `decide` does not restart it, so after each
  approval the person has to type `/vsdd-factory:drive` again.
- **What breaks:** A greenfield cycle takes about seven typed commands where the design promises three.
  User decision 7 (low touch) is met only in part.
- **Resolution:** At a human stop, `drive` sets `driver.json` to `paused` at that decision id instead of off.
  When the `decide` hook settles that decision and the driver was paused in this session, its returned context
  tells the main session to resume the driver loop. The briefing carries the same instruction.

### P2OPS-07 — `state.md` 2.11 keeps the old definition of "owned elsewhere" (OPS-06)

- **Severity:** minor
- **Locations:** `state.md` 2.11, last bullet ("A story branch that already exists on the remote is reported by
  the survey as owned elsewhere") against `state.md` amendment 16 and `BLUEPRINT.md` section 8, OPS-06 row.
- **Problem:** The section that the survey's builder (package S3) works from still has the definition OPS-06
  showed to be wrong. The new definition exists only in the amendment list.
- **What breaks:** If it is built as written, a branch this machine pushed just before a crash is refused as
  someone else's, and the story stays blocked after a cold start.
- **Resolution:** Replace the sentence with the amended definition: the remote head is neither the local
  branch's head nor an ancestor of it, and no local run record names the branch.

### P2OPS-08 — The checkpoint's mapping of blocked steps to decisions leaves out `review-blocked` (OPS-09)

- **Severity:** minor
- **Locations:** `pipeline.md` 2.6, the paragraph "Which decision a blocked step becomes" ("A step blocked with
  kind `gate`, `conflict` or `other` ... open nothing the first time") against the `review-blocked` row of the
  same section ("Opened the first time it happens"); the blocked-kind enumeration in `pipeline.md` 2.10.1 has no
  reviewer kind.
- **Problem:** A reviewer that refuses usually returns `blocked` with kind `other`. The one paragraph that
  specifies how the checkpoint maps stored results to decisions says that case opens nothing the first time.
- **What breaks:** Built from that paragraph, the checkpoint behaves as before: a reviewer refusal is retried
  and becomes `stuck` two runs later, which is exactly what OPS-09 item 5 reported.
- **Resolution:** Add to the paragraph: "A review loop whose verdict reason is `reviewer_blocked` (a review
  step returned `blocked`) opens `review-blocked` at once." Add a scenario test for it.

### P2OPS-09 — Agent input rows still name report paths the pipeline no longer passes (OPS-11)

- **Severity:** minor
- **Locations:** `agents-skills.md` 2.6 table: `implementer` "Is given: ... for a fix, the review report paths
  and finding identifiers"; `code-reviewer`, `security-reviewer`, `holdout-evaluator`, `ui-reviewer` "Is given:
  ... report path" and "Writes: One review report" (or "One evaluation report"). The paragraph above the table
  overrides this only for reviewers.
- **Problem:** Pipeline reviews write no reports (OPS-28), and a fixer receives the findings inline. The
  implementer row is not covered by the override paragraph, and every agent file says "If a required input is
  missing, return blocked".
- **What breaks:** If package A4 copies the row into the implementer's Inputs section, every fix step looks for
  report paths it will never be given and returns `blocked`, which is the failure OPS-11 reported.
- **Resolution:** Change the implementer row to "for a fix, the findings (inline) and their keys". For the four
  reviewer rows, mark report path and report as on-demand only, and give `holdout-evaluator` "Writes: nothing".

### P2OPS-10 — Remaining delivery and environment gaps from OPS-13

- **Severity:** minor
- **Locations:** `pipeline.md` section 7, R8 ("Delivery rebases and reruns the green gate when the base moved");
  `packaging.md` 2.10, the governed merge row (`--match-head-commit <sha>` "so GitHub itself refuses if the branch
  moved after review"); `state.md` 2.16, the `env start` row ("stopped when the step ends", with no mechanism
  given); `state.md` 2.12 (`commands.run` has no port parameter).
- **Problem:** R8 describes the delivery that OPS-13 replaced. The merge passes the reviewed commit id, which a
  clean rebase changes, even though the verdict now follows the `git patch-id`. Nothing stops a product
  process whose agent died or forgot `env stop`. Separate ports work only if the configured run command actually
  uses `VSDD_PORT`.
- **What breaks:** After a rebase caused by a conflict, GitHub refuses the merge and the story stalls. Leaked
  product processes pile up across 16 parallel evaluators and demo steps. Products that hard-code their port still
  collide on it.
- **Resolution:** Rewrite R8 to match `pipeline.md` 2.9.1. Have the merge pass the current head whose patch id
  matches the verdict's. Have `vsdd env start` record the caller's `agent_id`, so that `lifecycle-log` on
  `SubagentStop`, and the checkpoint, stop leftover processes. Let `commands.run` take a `{port}` placeholder or
  read `$VSDD_PORT`, and have `vsdd init` and `vsdd doctor` check it.

### P2OPS-11 — Configuration changes: the briefing rule still announces only `[hooks]`, and the fingerprint misses gate-bearing sections (OPS-14)

- **Severity:** minor
- **Locations:** `hooks.md` 2.11.8, fourth bullet ("a change to `[hooks]` since the last session") against
  `hooks.md` 2.6 and `state.md` amendment 2 (every gate-bearing section). The fingerprint list in both places
  leaves out `[[component]]` (per-component `commands` override `[commands]`), `[stops]`, `[release]` (publish
  commands), `[spikes]` and `[caps]`.
- **Problem:** The section that package H9 builds the briefing rule from still has the old scope. Several
  sections that weaken a gate when changed are not fingerprinted. `factory.toml` travels with the pushed state
  branch, so a change made on another machine or by a teammate arrives without notice.
- **What breaks:** A `[[component]]` entry whose `test = "true"` makes the green gate pass for that part of the
  tree, and no person is told.
- **Resolution:** In 2.11.8, say "a change to any gate-bearing section (2.6)". Add `[[component]]`, `[stops]`,
  `[release]`, `[spikes]` and `[caps]` to the fingerprint list in `hooks.md` 2.6 and `state.md` amendment 2.

### P2OPS-12 — `vsdd status --fix` is in the `read` group, open to every agent without a prompt (OPS-15)

- **Severity:** minor
- **Locations:** `hooks.md` 2.7 `[callers]` (`read` includes `status`); `BLUEPRINT.md` 4.9 (`status [--fix]`
  listed as a read-only view for any caller); `state.md` 2.10 (`vsdd status` flushes, and `--fix` remounts,
  releases a lock whose holder looks dead, and removes worktrees and evidence); `hooks.md` 2.11.10 (`auto-allow`
  permits read subcommands for the main session without a prompt).
- **Problem:** OPS-15 listed `status --fix` among the state-changing commands any agent could run without a
  prompt. The fix left it in the `read` group.
- **What breaks:** Any builder or reviewer with a shell can release a lock and remove worktrees, and it needs no
  prompt to do so. Combined with P2OPS-01, it can release a live run's lock that looks stale.
- **Resolution:** Let the caller check look at flags. Put `status --fix` and `flush` in the `author` group
  (main session) and in the clerk's `pipeline` group, keep plain `status` in `read`, and correct the blueprint 4.9
  row to match.

### P2OPS-13 — The spike runner still has an unlogged way out through the shell (OPS-16)

- **Severity:** minor
- **Locations:** `hooks.md` 2.7, the spike-runner row (`read_deny = ["holdout", "reviews", "runs"]`, no
  `outside`) and the note after it on `outside`; `hooks.md` 2.11.1, `secrets` row (the logging and URL secret
  check apply only to `WebFetch`, `WebSearch` and the research tools; the environment-dump list has no
  `export -p`); `agents-skills.md` 2.5 (the spike runner has WEB, LOOKUP and SHELL).
- **Problem:** OPS-16 named the researcher and the spike runner as the two agents that read untrusted pages.
  The fix walls the researcher, which has no shell. The spike runner keeps read access outside the project, and it
  can send data out in a `curl` or `wget` GET URL that is neither logged nor checked. Only upload flags are blocked.
- **What breaks:** A prompt-injected spike runner can read a credential file that is not on the secret glob
  list and send it out unseen. The rest of OPS-16 is fixed.
- **Resolution:** Add `outside` to the spike runner's `read_deny`; its experiments still reach toolchain files
  through the shell. Apply the `web.call` log and the URL secret check to `curl`, `wget` and `http` URLs in Bash
  for agent rows. Add `export -p` and `declare -x` to the environment dumps.

### P2OPS-14 — The bundled browser server is not actually started with file access off (OPS-17)

- **Severity:** minor
- **Locations:** `packaging.md` 2.4, `.mcp.json` (`"playwright": { "command": "npx", "args": ["-y",
  "@playwright/mcp@<exact>"] }`, with no option) and the note after it (the plugin's copy of a server is dropped
  as a duplicate when the user already runs one); against `hooks.md` 2.11.2, the browser row ("The plugin's own
  Playwright server is also started with local file access off") and the `BLUEPRINT.md` section 8 OPS-17 row.
- **Problem:** The second layer of the OPS-17 fix is in the hooks design and the blueprint, but not in the
  document that owns `.mcp.json`. On the user's machine the user's own Playwright server may survive in place of
  the bundled one, and its `file:` access is unrestricted. `browser_click` on a link to a `file:` address is not
  matched, so only `browser_navigate` is checked.
- **What breaks:** Only the hook's navigate check stands between a walled agent and a `file:` page reached by
  clicking through.
- **Resolution:** Add the server option that refuses `file:` navigation to the pinned `.mcp.json` entry, and
  have L-MCP check that it is there. Add a probe (section 10) for what happens to the bundled server when the user
  already runs one. If the user's server survives, add `browser_click` and `browser_tabs` to the matcher and treat
  a resulting `file:` page as a read, or have walled agents list only the plugin's spelling.

### P2OPS-15 — The offer to archive old process sections is missing from `setup` (OPS-19)

- **Severity:** minor
- **Locations:** the `BLUEPRINT.md` section 8 OPS-19 row ("setup offers to move conventions and archive old
  process sections") against `BLUEPRINT.md` 4.5 and the `agents-skills.md` 2.12 `setup` row (they offer only to
  move build conventions); `packaging.md` 2.1 and 2.9 (the cutover "does not edit a project's `CLAUDE.md`").
- **Problem:** Agents now leave out the project's `CLAUDE.md`, but the main session still loads it. In `prism`
  and `pregolya` that is about 600 lines of old process rules: route work to the state-manager, never edit
  `STATE.md`, run 3-clean convergence. The main session is the driver.
- **What breaks:** The main session follows old rules while it drives workflows. For example, it tries to
  dispatch agents that no longer exist, or it declines to run `vsdd` commands, and two of the eight projects pay
  for that on every session.
- **Resolution:** Add the archive offer to the `setup` row in `agents-skills.md` 2.12 and blueprint 4.5. With the
  person's confirmation, it moves sections that use old process terms into `docs/archive/CLAUDE-pre-1.0.md`, so a
  person can still read them. Have the cutover report point at that offer.

### P2OPS-16 — The sandbox requirements are never stated, and `doctor --fix` cannot install the Python it needs to start (OPS-22)

- **Severity:** minor (partial, with one new inconsistency)
- **Locations:** `BLUEPRINT.md` section 8 OPS-22 row ("the sandbox needs are stated"); `packaging.md` amendment
  8 ("The documentation states what a sandboxed Bash tool must allow") and 2.10 `vsdd doctor` row: no document
  lists those needs. For `doctor --fix`: `BLUEPRINT.md` 4.1 ("Only `vsdd doctor --fix` installs a Python"),
  `hooks.md` 2.2 (the launcher runs every entry with `--no-python-downloads --python '>=3.11'`), `packaging.md`
  I3 and 2.10.
- **Problem:** OPS-22 asked for the sandbox needs to be specified, and the fix only says the documentation will
  state them. `vsdd doctor` is Python, started by the same launcher. On a machine with no Python 3.11 or newer,
  which is the case `--fix` exists for, the launcher exits before `doctor` runs.
- **What breaks:** Package T1 has no specification for its sandbox check, and package D2 has nothing to tell the
  person to allow, so in a sandboxed session every survey still fails. A person told to run `vsdd doctor --fix`
  gets the launcher's error, not a Python. The launcher's own fix line (`uv python install 3.12`) is correct.
- **Resolution:** List the sandbox needs in `packaging.md` 2.10: write access to `.factory/`, `.worktrees/`,
  the uv cache and `~/.cache/vsdd-factory/`; network to the git remote and `api.github.com` for delivery. Have
  `vsdd doctor` test each one. Either have `launch.sh` run `uv python install 3.12` itself for `doctor --fix`
  before Python starts, or drop the claim and name the terminal command as the one fix in 4.1, `hooks.md` 2.2
  and `packaging.md`.

### P2OPS-17 — The low agent-count estimate still sets the end-to-end budget (OPS-25)

- **Severity:** minor
- **Locations:** `pipeline.md` section 7, R12 ("Typical story is 4 to 6 agents"); `packaging.md` 2.6, story
  tier, "Lean" ("fail the run above a budget (1.5 times the figure the pipeline design gives for a typical
  story)"); against `packaging.md` amendment 10 ("starting from 12 agent starts for a story") and `BLUEPRINT.md`
  section 10 item 22 (8 to 12).
- **Problem:** The body text still derives the story-tier budget from the estimate that OPS-25 showed to be
  half the realistic figure. The corrected number appears only in the amendment list.
- **What breaks:** Built from 2.6, the story tier fails any normal run above 6 to 9 agent starts.
- **Resolution:** In R12, use 8 to 12 per story and 70 to 90 per six-story wave. In 2.6, set the budget to
  12 starts per story until package E2 has measured a real figure.

### P2OPS-18 — Child stories start their spike count at zero, so refused spikes still cost an agent (OPS-27)

- **Severity:** minor
- **Locations:** `pipeline.md` 2.8, the limits table ("Spikes per run ... The wrapper (per script) and
  `vsdd spike open` (per run id, so parent and children share it)"); `pipeline.md` 2.9.1 (a child receives
  `position`, `run_id`, `project_dir` and `limits`, and skips its own survey); 2.10.8 (`spikes_used` is a
  run-level survey field); `pipeline.md` amendment 16 and the `BLUEPRINT.md` section 8 OPS-27 row ("refusal
  before an agent starts").
- **Problem:** Children run in parallel and cannot share a script counter. None of them receives the run's
  `spikes_used` or a share of the limit, so across children the run-wide limit is enforced only inside
  `vsdd spike open`, after a spike runner has started. That is the hole OPS-27 named.
- **What breaks:** With 16 stories running in parallel, a run that reaches its spike limit still pays an agent
  start for each refused spike: up to the per-child local limit, times the number of children.
- **Resolution:** The parent passes each child `spikes_per_run` as a share of what remains (at least 1), along
  with the parent's `spikes_used`, so the child's wrapper refuses locally. `vsdd spike open` stays as the
  backstop.

### P2OPS-19 — Bookkeeping residues: flush wording, review identifiers, and register limits in CI (OPS-28)

- **Severity:** minor
- **Locations:** `state.md` 2.10 ("flush already ran at the last `vsdd` command") and 2.17, first bullet ("state
  is committed at every `vsdd` command"), against `state.md` 2.3 and amendment 11; `state.md` 2.4, identifiers
  table, "Review, evaluation" row (path `<cycle>/<unit>/<lens>-<n>`, allocated by `vsdd review path`), against
  2.5 (on-demand reports at `reviews/<yyyy-mm-dd>-<lens>-<n>.md`, pipeline reviews have no path, evaluations
  written by code); `state.md` 2.16, `vsdd validate` (it checks "register limits", and it is used in CI per
  2.3) against amendment 10.
- **Problem:** Two sentences still describe the flush-on-every-command behaviour OPS-28 removed. The identifier
  table gives a third path scheme for reviews. It is unspecified whether a register limit makes `vsdd validate`
  exit non-zero outside a gate.
- **What breaks:** If built literally, the flush points drift back toward hundreds of commits per wave, and
  `vsdd review path` is built to a scheme no other section uses. In a project that runs `vsdd validate` in CI, the
  31st lesson turns CI red, `story deliver` waits on CI and gets `ci_failed`, and a repair loop starts that no
  builder can fix: the governance-blocks-delivery failure OPS-28 described.
- **Resolution:** Correct the two sentences to the flush points in 2.3. Make the identifier row match 2.5.
  State that register limits are warnings in `vsdd validate` output with exit 0, everywhere, including CI.

### P2OPS-20 — The script skeletons still encode the pre-fix behaviour of nine operations findings (cross-cutting)

- **Severity:** minor
- **Locations:** `pipeline.md` 2.11.1 to 2.11.3. In order of the findings they reverse:
  - OPS-01: the `ci_pending` loop in `story.js` (`waits < 6`).
  - OPS-05: `converge()` starts at pass 0 and never reads `rounds_used`.
  - OPS-06 and OPS-08: `brief()` adds the earlier-work line only when `n > 1`, and ends with the identity but
    not the closing command.
  - OPS-09: `stepSchema` does not require `data`; `relay()` makes no retry; there is no `try`/`finally`;
    `wave.js` returns `deferred: []` from its `catch`; a reviewer that returns `blocked` becomes `failed` with
    reason `no_result`.
  - OPS-12: `relay()` says "Run exactly this command once in ${RUN.dir}".
  - OPS-23: the arbiter dismisses a finding without checking that it was disputed, and a pass with coverage
    gaps but no findings never calls the fixer.
  - OPS-26: `STORY_WORST = 30`, and children receive `limits: RUN.limits` with `agents: 900`.
  - OPS-27: research spikes run without isolation on the builder tier, and spike findings are not carried across
    `gated()` attempts.
- **Problem:** Section 2.11 says the skeletons predate the blueprint and points to the amendment items. But
  `BLUEPRINT.md` 4.14 names `judge()` and `converge()` in the prelude as the single source of the convergence
  rule, and the only rendering of that code anywhere in the design is the pre-fix one. The amendment for the
  `-C` prompt is in no listed item at all (see P2OPS-02).
- **What breaks:** Packages B4, W2 and W3 start from the only code there is. The conformance fixtures (package
  G3) are cut from whatever gets ported, so the Python verdict would then agree with a prelude that still
  dismisses undisputed findings and loops on coverage gaps. OPS-23 named exactly this risk ("the tool will copy
  the same behaviour through the P4 fixtures").
- **Resolution:** Update the three skeletons to the amended behaviour and re-run the six story cases and three
  wave cases in Node. If that is not done, write the amended behaviours as conformance fixtures now, in
  `packaging.md` 2.6, layer 2b: dismissal only of disputed findings; a coverage gap calls the fixer; the cap
  counts `rounds_used`; `blocked` from a reviewer gives `reviewer_blocked`; children get `story_agents`; research
  spikes are isolated on the planner tier; findings carry across attempts.

## Checked and found sound

- **OPS-01:** detached jobs, `job wait --max 540` with a 600000 ms Bash timeout, one clerk per CI wait,
  `stallMs` on waiting steps, the six-wait limit dropped, and an eleven-minute probe. These are consistent across
  `pipeline.md` 2.5 and 2.9.1, `state.md` 2.12 and 2.16, `agents-skills.md` 2.4, `packaging.md` amendment 10,
  and `BLUEPRINT.md` 4.9 and section 10 item 6.
- **OPS-04:** syntax checked at the write, existence checked at the gates and at approval. Consistent in
  `state.md` 2.4, `hooks.md` 2.11.3 and `pipeline.md` 2.9.3.
- **OPS-08:** `compaction-snapshot` branches on `agent_type` (`hooks.md` 2.11.9). Step prompts repeat the
  identity and the closing command (`pipeline.md` 2.5). Probe 12 is in section 10.
- **OPS-10:** the hook blocks an ungrounded review once and then stores it as advisory, resolving paths through
  `unit_tree` with three lines of drift. The tool applies the same rule. See `hooks.md` 2.11.7 and
  `pipeline.md` 2.10.2.
- **OPS-20:** dependency additions and upgrades beyond `dependencies.toml` need a `merge` decision, batched per
  maintenance run (`pipeline.md` 2.6, `state.md` 2.12, D15).
- **OPS-21:** `awaiting_merge` opens a `merge` decision with the link. Readiness is a chore story in `E-0`.
  `--settle` creates a trailing wave, and a run returns `needs_human` when all that is left waits on decisions
  (`pipeline.md` 2.9.1, 2.9.2 and 2.9.5).
- **OPS-23:** coverage gaps become `coverage` findings; `not_checkable`, `waive-coverage`, dismissal only of
  disputed findings, and `direct` as exactly one more round are in `pipeline.md` 2.6 and 2.7 and in the
  choice table of `state.md` 2.8. The skeleton is the exception (P2OPS-20).
- **OPS-24:** the quarantine variable works only after repeated `rule.error` events and is announced in every
  briefing (`hooks.md` 2.6, D12).
- **OPS-26:** each child receives `story_agents` (40), and the harness models the counter shared by parent and
  children (`pipeline.md` 2.5, `state.md` 2.12, `packaging.md` 2.6 layer 2). The skeleton is the exception
  (P2OPS-20).
- **OPS-29:** `status.json` lives under `local/` and is rebuilt on demand, and notes are a tracked file. No
  document still points at the old path.
- **OPS-30:** `merge-guard` blocks commits, amends and branch-moving resets on a checked-out integration or
  release branch, and names `/vsdd-factory:feature` for the main session (`hooks.md` 2.11.4).
- **OPS-31:** `setup` offers the attribution setting and `vsdd doctor` checks it (`agents-skills.md` 2.12,
  `packaging.md` amendment 12 and 2.10, D11).
- **Partial fixes whose core holds:** OPS-02 (digest and ordering), OPS-03 (the `decide` skill, the prompt
  hook, the shim), OPS-05 (rounds persist, and the route row is reordered), OPS-06 (verdict records, `dirty`,
  `open_pr`, adoption of an open pull request, kills mid-step and mid-wait), OPS-07 (atomic take, one scope
  rule, liveness owned by the hook), OPS-09 (required `data`, `try`/`finally`, the universal test), OPS-13
  (patch-id binding, the semaphore, `[commands.env]`), OPS-14 (configuration not writable by any tool call;
  the settings, plugin-disable and `claude config` routes blocked), OPS-15 (`[callers]`, the person-only list, the
  sender check on step results, reserved event types, no direct calls into the Python package), OPS-16 (the
  wider secret lists, the `outside` wall for agents without a shell, checks on web tools), OPS-17 (the matcher
  and `file:` addresses read as paths), OPS-18 (JUnit reports and freezing by test identity), OPS-19
  (`omitClaudeMd` on every agent, linted), OPS-22 (a block message that works in a terminal, one download
  policy), OPS-25 (one clerk per CI wait, batches of 12), OPS-27 (isolation, the question hash), OPS-28 (no prose
  reports from pipeline reviews, fewer flush points in 2.3, register caps as status problems).
