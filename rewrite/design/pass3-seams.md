# Review pass 3: the seams between the six documents

Date: 2026-11-24. Branch `rewrite/v1`. Reviewer lens: names, paths, schema fields, identifiers, configuration
keys, commands, caller permissions and responsibilities that differ between two documents, or between a
document and blueprint section 4; and each document's stated needs against what the others provide. Known
items R1 to R5 (`pass2-closeout-3.md`) are not repeated. Line numbers are as of this reading.

## Verdict

The documents agree far more than they disagree. Most of what the last two rounds changed is consistent in
every place it appears: the route kept in `cycle.json`, the `guidance` class and its lock, the digest
exclusions, the `refreeze` caller group, the D4 scope switch and the setup order, the walled key, the seven
product classes and their precedence, and the admission commands returning a unit. The seams that remain
break things. I found 24 problems: 1 blocker, 8 major and 15 minor, and 14 of them lie on the thin-slice path.
The blocker is in `feature`: `vsdd feature route` is told to read a "delta record" that has no path, no class
and no writable home, because the specification lock refuses the architect's writes during the delta step.
The majors on the slice path are these:
- The verdict function is told to count a pass only when its agent type owns "the pass's method", but no
  record carries a method.
- "The cycle's base", the commit a story branch is cut from, is never defined, and `cycle.json` has only
  `baseline`, a release tag.
- Local delivery mode has no defined way to merge into an integration branch that is checked out in the
  person's own checkout.
- The example wall rows deny builders `git status`, `diff` and `log`, which D4 approves. A row with no
  `commands` key gets every git family.
- `state.md` calls its example the whole key list, which makes the `[hooks]` keys of `hooks.md` 2.6 errors
  and so breaks D12.

The other majors are not on the slice path:
- "Live run" carries two meanings that disagree.
- One story `method` key is asked to choose the methods of two stages.
- The `core` scope approves every configured command, not only the build and test commands D4 named.

## Findings

### P3SEAM-01. `vsdd feature route` has no input it can read

- Severity: blocker. Slice path: no.
- Locations: `pipeline.md` 2.9.4, delta-and-route row ("the tool derives the route from the delta record the
  step just wrote"); blueprint 4.9, `feature route` row ("Derive the route from the stored change analysis"),
  and 3.3 step 2; `state.md` 2.16, `vsdd feature route` row (arguments `--run <id>` only); `agents-skills.md`
  2.6, architect row (Writes lists no delta record) and story-writer row ("the delta record path");
  `state.md` 2.4, the lock row; `hooks.md` 2.11.3, `artifact_frozen`; `pipeline.md` 2.6 and 2.9.5 ("the
  stories the change analysis names").
- Problem:
  - No document gives the delta record a path, an artifact kind or a class. The only candidate is an
    assessment of kind `impact` in `specs/architecture/assessments/`, which is in `spec-arch`.
  - During the delta step the specification set is locked. The latest `spec` decision is approved: a
    release, an ingest or an import is a precondition of `feature`. The run holds no amendment yet, because
    only `vsdd feature route` marks one. So the architect's write is refused with `artifact_frozen`, and it
    can write nothing else except the temporary directory.
  - The route command is the step's closing command. It runs before the step's `StructuredOutput` call, so
    the step record that holds the delta data does not exist yet either.
  - Later readers need the change analysis too: the story-writer on a fast route, the `spec` packet on the
    standard route, and `plan` in delta scope. The story-writer is denied `runs`.
- What breaks: every `feature` run, so every brownfield change, every bug fix and every feature after a
  release. The route cannot be derived, and the story-writer and the delta-scope plan have nothing to read.
- Resolution:
  - Give the command a file input, as the spike commands have: `vsdd feature route --run <id> --delta-file
    <path>`. The architect writes the delta data (`pipeline.md` 2.10.8) as JSON in the temporary directory.
  - The tool validates the file and copies it into the cycle's tracked record (for example
    `cycles/<cycle>/units/feature.json`). That record is the "stored change analysis". The story-writer, the
    packet and `plan` read it from there.
  - State this in `pipeline.md` 2.9.4, blueprint 4.9, `state.md` 2.8 and 2.16, `agents-skills.md` 2.6 (the
    architect writes the file; the story-writer is given the record's path) and the `delta-analysis` method.

### P3SEAM-02. The check that a review pass came from its reviewer has no method to check

- Severity: major. Slice path: yes (step-results, the review verdict).
- Locations: blueprint 4.10, Verdict row, and 5.4 ("the reviewer that owns the pass's method"); `pipeline.md`
  2.9.1, review row, 2.10.3 and 2.10.1 (the step identity is run, workflow, name, subject and attempt);
  `hooks.md` 2.11.7 step 2 ("no generated step-name table"); `state.md` 2.8, Step record row (it carries the
  agent type and the head) and S7 `store_step(project, result, agent_type, head)`; `packaging.md` 2.5 and
  layer 2b, the fixture "A reviewer pass stored with a builder's agent type".
- Problem:
  - The tool must count a pass only when the stored agent type is "the reviewer that owns the pass's
    method". No record holds the method: the step identity has no method field, and the hook stores only
    the agent type and the head.
  - Step names are built at run time and are the only other clue. In `story.js` they name the agent
    (`review-adversary-p1`, `fix-test-writer-p1`, `arbitrate`). In `wave.js` they name neither the agent nor
    the method (`integration-review-r1`, `security-r1`, `holdout-r1-b1`).
  - No document gives the step-name grammar that the tool's verdict function must parse. That function
    needs it to find a unit's passes, fixes and arbitration in the step records, and to order them.
- What breaks: G3 cannot build the verdict as specified, and the conformance fixture cannot be written. A
  builder will invent a name parser. The anti-forgery guarantee then depends on an undocumented convention
  that breaks the first time a step is renamed.
- Resolution:
  - Add `method` to the step identity. `brief()` puts it in the `step` object the agent echoes back, and
    `pipeline.md` 2.10.1 adds it to the schema. The hook stores it with the record.
  - The verdict counts a record as a pass of a perimeter when its method is one of that perimeter's review
    methods (`pipeline.md` 2.9.13) and the stored agent type is the agent whose file lists that method.
    Fixes and arbitration are found the same way: `test-fixes`, `fix-findings`, `arbitrate`.
  - Add the pass or round number to the identity as well, so ordering does not depend on parsing names.
  - Update blueprint 4.10, `hooks.md` 2.11.7, `state.md` 2.8 and S7, and the `packaging.md` fixture.

### P3SEAM-03. "The cycle's base" is undefined, and `cycle.json` offers only `baseline`

- Severity: major. Slice path: yes (the survey worktree, the red and green gates).
- Locations: `pipeline.md` 2.9.1 ("creates the branch `story/<id>` from the cycle's base (the release tag, on
  `hotfix/<version>`, for a critical fix)"); `state.md` 2.16, `gate plan` row ("from the cycle's base"); the
  story unit field `base` (`pipeline.md` 2.10.8); blueprint 4.13 and `state.md` 2.8 (red evidence "between
  the story's base and its head"); `state.md` 2.16, `gate green` (the diff "from the story's base");
  `pipeline.md` 2.9.1 ("New tests are the test identities missing from the report of the base commit");
  `state.md` 2.8, Cycle record (`baseline`: a release tag, an ingest or an import).
- Problem:
  - `cycle.json` has no `base`. Its nearest field is `baseline`, which is a release tag, and the hotfix
    parenthesis ("the release tag") makes `baseline` look like the intended source.
  - `pipeline.md` 2.9.4 instead speaks of a "base branch". `state.md` 4 answers `packaging.md` S2 with
    `branches.integration`.
  - Nothing says which commit is used: the local or the remote head of the integration branch, and when.
    Nothing says where the chosen commit is recorded for the check that "its branch descends from the
    recorded base".
- What breaks: if the base is read as `baseline`, every story in a feature cycle is cut from the last release
  tag instead of the integration head. The green diff then includes every earlier story of the cycle, and the
  red gate counts their passing tests as new, so it fails "at least half are red". For a first release
  `baseline` is empty, so the slice's builder must guess.
- Resolution:
  - Define the base once, in `state.md` 2.16 (the worktree library) and `pipeline.md` 2.9.1. It is the head
    of `branches.integration` (after a fetch in `pr` mode, the remote head) when the worktree is created. In
    a hotfix cycle it is the release tag the cycle names.
  - The commit is recorded in the run record and in the story's closure. The unit's `base`, the descent
    check, the red-gate comparison and the green diff all use that recorded commit.
  - Replace "the cycle's base" with that definition in both places.

### P3SEAM-04. Local delivery mode cannot merge without touching the person's checkout

- Severity: major. Slice path: yes (delivery in local mode).
- Locations: `pipeline.md` 2.9.1 ("With `delivery.mode = "local"` (no remote) it merges locally and skips the
  pull request and CI"), the only description of the mode; `state.md` 2.3 and 1 ("the person's own checkout
  is never switched, pulled or committed to"); `hooks.md` 2.11.4, `merge-guard` ("in local delivery mode,
  where the integration branch is the product branch"); blueprint 4.12 and 9.2 G4a.
- Problem:
  - With no remote, the integration branch is normally the branch checked out in the person's main
    checkout.
  - Git will not check that branch out in `.worktrees/_integration/`.
  - Moving the ref with `update-ref` or a merge elsewhere leaves the person's index and files behind HEAD,
    so they show up as staged reversions. Merging in the main checkout breaks the rule that the person's
    checkout is never touched.
  - No document picks a behaviour.
- What breaks: G4a, on the slice path, has no specified merge. Any choice silently breaks either the
  person's checkout or the "never touched" rule.
- Resolution:
  - State the local-mode merge in `pipeline.md` 2.9.1 and `state.md` 2.16 (`story deliver`), and have
    `vsdd doctor` and `vsdd status` report a violation. One workable rule: the merge happens in a tool-owned
    worktree on a detached head, and the ref is advanced only when the integration branch is not checked out
    in another worktree.
  - When it is checked out there, either fast-forward that checkout only if it is clean and at the old head
    (a stated exception to "never touched"), or return `awaiting_merge` with the one command for the person.
  - Record the chosen rule beside `merge-guard`'s local-mode note.

### P3SEAM-05. Git command families: builders are denied what D4 approves, and an absent key grants everything

- Severity: major. Slice path: yes (test-writer and implementer rows, `bash-git`).
- Locations: `hooks.md` 2.7, implementer and test-writer rows (`commands = ["git-commit"]`), the key table
  ("`*` means any, and is the default when the key is absent"), the `other` row ("the strictest treatment")
  and the devops and verifier rows (no `commands` key); `hooks.md` 2.11.1 (families: `git-read` is "every
  other `git` and `gh` command"; `bash-git` blocks "a command family the caller's row does not grant");
  `hooks.md` 2.11.10 and blueprint 2.1 D4 and 5.4 (`core` approves "`git add`, `commit`, `status`, `diff` and
  `log` inside those worktrees"); `agents-skills.md` 2.9, command scopes (only the holdout-evaluator is
  denied git; "`git push`, `gh pr create`, worktree add ...: inside the pipeline no agent runs them");
  `packaging.md` 2.6, smoke probe 7 (a builder step "runs `ls` and `git status`").
- Problem:
  - `git status`, `git diff` and `git log` fall in `git-read`, which the implementer and test-writer rows do
    not grant, so `bash-git` blocks them. D4's own list of approved calls cannot run.
  - Every row without the key (devops, verifier, technical-writer, demo-recorder, codebase-analyzer,
    spike-runner, `other`) gets every family: `git-push`, `gh-pr`, `git-worktree`, `git-integrate`. That
    contradicts `agents-skills.md` 2.9 and the "strictest" `other` row.
  - `git-read` as defined also includes `git checkout`, `git switch`, `git fetch` and `gh api`.
- What breaks:
  - The slice's builders are refused routine read-only git calls.
  - Smoke probe 7 sees a block where it expects no prompt.
  - Devops or a built-in agent dispatched by the main session can push story branches and open pull
    requests, so a pushed branch with no tool-opened pull request can be read as "owned elsewhere".
- Resolution:
  - In `hooks.md` 2.11.1, define `git-read` as the reading commands only: `status`, `diff`, `log`, `show`,
    `rev-parse`, `ls-files`, `blame`, `grep`. Everything else that names no family is refused unless a row
    holds `"*"`.
  - Make an absent `commands` key mean `["git-read"]`. Give every builder row `["git-commit", "git-read"]`
    and the main row `["*"]`.
  - State the same in `agents-skills.md` 2.9 command scopes.

### P3SEAM-06. The configuration loader's "whole key list" omits the `[hooks]` keys, so D12 relaxations are errors

- Severity: major. Slice path: yes (`config.py` in the shared core).
- Locations: `state.md` 2.12 ("The example above is the whole key list: every section and every key the loader
  accepts appears in it"; "Unknown keys and wrong types are errors"; the example's `[hooks]` holds only
  `main_session_wall` and `[hooks.rules.auto-allow] scope`), and the same section's ownership row (`[hooks]`:
  `main_session_wall`, `log_retention_days`, and `[hooks.rules.<rule>]` tables, `hooks.md` 2.6); `hooks.md` 2.6
  (`log_retention_days`, `[hooks.rules.tests-first] strength`, `[hooks.rules.bash-git] attribution`,
  `[hooks.rules.bash-destructive] allow_pr_close`; unknown keys under `[hooks]` fall back to defaults), 2.11.1
  ("the configured `secret_globs`"), 2.11.10 (`purity-advice`'s configured pure-core paths and side-effect
  patterns), 2.12; blueprint 2.1 D12.
- Problem:
  - The two documents disagree on what the loader accepts.
  - A loader built to `state.md` 2.12 rejects `strength`, `log_retention_days` and every per-rule key.
    `secret_globs` and the `purity-advice` patterns have no key anywhere.
  - `hooks.md` assumes those keys are read.
- What breaks: D12. A person setting a gate to `advise` or `off` gets a configuration error, and "an invalid
  value never relaxes a gate". The per-rule settings `hooks.md` documents cannot be set. The loader is built
  in B3, on the slice path.
- Resolution:
  - Add the `[hooks]` keys to the `state.md` 2.12 example: `log_retention_days`, and per rule `strength`
    plus the rule-specific keys `attribution`, `allow_pr_close`, `secret_globs`, `pure_paths` and
    `side_effect_patterns`, with their defaults.
  - Alternatively, state there that `[hooks]` is validated by the hook design's schema, with `hooks.md` 2.6
    as its full list. Use the same key names in `hooks.md` 2.11.1 and 2.11.10.

### P3SEAM-07. "Live run" has two needs and no definition

- Severity: major. Slice path: no.
- Locations: `hooks.md` 2.11.7 step 2 ("A record is accepted only when `step.run` names a live run"); blueprint
  5.4; `state.md` 2.4 and S7 `spec_locked` ("no live run record holds an amendment"); `state.md` 2.11 (a lock
  holder is alive while `seen` is recent, the only liveness defined); `pipeline.md` 2.5 and `state.md` 2.11
  (`discover`, a report-only `maintain` and `spike` take no lock).
- Problem:
  - Liveness is defined only for a lock holder. Under that reading the steps of `discover` and of a
    report-only `maintain` belong to no live run, so their records are refused.
  - The `discover` checkpoint needs the score records to build the `ideas` decision.
  - `vsdd spike open --step` needs the asking step's stored record, so a spike asked for from those runs is
    refused.
  - The opposite reading, "not yet checkpointed", would leave the specification unlocked after an amendment
    run crashes, until the run records are pruned 30 days later.
- What breaks: discovery and sweeps lose every step result, or the lock stops relocking after a crash. Which
  one depends on the builder's guess.
- Resolution: define it once in `state.md` 2.8. A run is live when its record has no checkpoint mark, and
  either it holds the lock with a fresh `seen` or its workflow takes no lock and it was opened within its time
  limit. Amendment runs always take the lock, so for `spec_locked` the definition reduces to lock liveness.
  Cite the definition in `hooks.md` 2.11.7 and blueprint 5.2 and 5.4.

### P3SEAM-08. One story `method` key is asked to choose both stages' methods, and chore stories have two defaults

- Severity: major. Slice path: no (the slice's story is a `feature` story with no `method` key).
- Locations: blueprint 4.6 ("`method`, which picks another method owned by the same agent"); `pipeline.md`
  2.9.13, story tests row ("`ui-tests` or `e2e-tests` when the story's `method` key says so"); `pipeline.md`
  2.11.2 `story.js` (`KIND`, and `u.method || K.impl[1]` for the implement stage only; `chore` defaults to
  `repo-setup`); `hooks.md` 2.7 and 2.11.2 (`@<method>` "counts only in a story whose method is that one");
  `state.md` 2.5, story frontmatter (`method` optional); blueprint 4.14 (no single source for the kind to
  agent and method table); `packaging.md` L-KIND.
- Problem:
  - A story whose `method` is `ui-tests` sends that test-writer method to the implementer.
  - A chore story without a `method` key is implemented with `repo-setup` by the script. The hook, reading
    the absent key, denies the `source:create@repo-setup` grant, so the scaffold story cannot create
    `src/main.rs`.
  - A CI chore story without the key is told to use `repo-setup` and may create source.
  - The agent and method table exists twice: the 4.6 table, and `story.js`'s `KIND`, with no generation or
    check. `artifacts.toml` holds only the red-gate column.
- What breaks: the scaffold story that P2SEAM-01 fixed, whenever the story-writer omits the key; and every
  story that needs a UI or end-to-end test method.
- Resolution:
  - Put the full 4.6 table in the story-kind table of `artifacts.toml`: tests agent and default method,
    implement agent and default method, red gate.
  - Split the key into `tests_method` and `impl_method`. The survey and the admission commands resolve each
    key or its kind's default into the unit.
  - `story.js` uses the unit's values instead of `KIND`, and the hook resolves `@<method>` from the same
    resolved value.
  - Update blueprint 4.6 and 4.14, `pipeline.md` 2.9.13, `state.md` 2.5 and `hooks.md` 2.7.

### P3SEAM-09. The `core` scope approves every configured command; D4 adopted only build and test

- Severity: major. Slice path: no (`auto-allow` is not in the slice).
- Locations: blueprint 2.1 D4 ("the project's configured build and test commands"), 2.4, 5.4 ("the project's
  configured commands"), 11.3 item 1 (lists only two additions to the adopted list: argument prefixes and
  workflow starts); `hooks.md` 2.11.10, `core` row ("The project's configured commands (`[commands]` and any
  `[[component]]` commands)").
- Problem: `core` also approves `lint`, `format_check`, `run` (a long-lived product server), `ui_check`,
  `docs_check`, `mutation`, `scan`, `audit` (which may reach the network), `bench` and `proofs`. The design
  keeps `extended` off by default because it "goes beyond the list in decision D4", but this third widening
  sits inside `core`, and 11.3 does not show it to the person.
- What breaks: the user's D4 answer. Approvals the user did not adopt are on in every project set to `core`.
- Resolution: in `hooks.md` 2.11.10 and blueprint 5.4, limit `core` to `commands.build` and `commands.test`
  (and the same keys of `[[component]]`), matched as a prefix. Move the other configured commands to
  `extended`. Alternatively, list the widening in 11.3 as a choice the setup question explains.

### P3SEAM-10. The checkpoint command takes no `--subject`, but both skeletons pass it

- Severity: minor. Slice path: yes (the standalone story checkpoint).
- Locations: `pipeline.md` 2.11.2 `end()` (`vsdd checkpoint story --run ... --subject ${sid}`) and 2.11.3 `close()`
  (`--subject ${W}`); blueprint 4.9 and `state.md` 2.16 (`checkpoint <workflow> --run <id> [--outcome failed]`);
  `pipeline.md` 5.1 S7.
- Problem: a parser built to the command table rejects the option.
- What breaks: the checkpoint returns a usage error. The clerk relays `{ok: false}`, and the run ends without
  closures or a released lock until the session ends or the lock goes stale.
- Resolution: remove `--subject` from the two skeletons, because the run record holds the subject.
  Alternatively, add `[--subject <id>]` to the three tables, checked against the run record.

### P3SEAM-11. The scripts read fields the schemas do not carry: the unit's `review_rules` and the wave's `rounds_used`

- Severity: minor. Slice path: yes (`review_rules`).
- Locations: `pipeline.md` amendment 6 (per unit `review_rules`); 2.10.8, Story unit row (no `review_rules`) and
  Survey row (`config` holds `review_rules`; no top-level `rounds_used`); `story.js` (`u.review_rules`);
  `wave.js` (`s.rounds_used`); blueprint 5.1 ("Gate rounds ... counted across runs").
- Problem:
  - A unit from `gate plan --story`, `story new-fix` or the wave survey carries no `review_rules`, so
    every story reviewer is told "Review rules: none".
  - The survey has no wave-level `rounds_used`, so `wave.js` always starts the cap at 0.
- What breaks: the story reviewers never get the project's review rules. The wave loop runs more rounds than
  the cap allows across runs, and the tool's verdict then contradicts the script, which wastes rounds.
- Resolution: add `review_rules` to the Story unit field list, filled by every command that returns a unit.
  Add `rounds_used` for the run's own unit (wave, verify) to the Survey fields. Update `pipeline.md` 2.10.8
  and `state.md` 2.16.

### P3SEAM-12. Two limit names the survey returns are unknown to the prelude, and an unknown name fails the run

- Severity: minor. Slice path: yes (the prelude, harness universal test 9).
- Locations: blueprint 4.12 and `state.md` 2.12 (prelude names include `gate_minutes` and `ci_minutes`; "A
  test fails on any unknown limit key"); `pipeline.md` 2.11.1 `RUN.limits` (no `gate_minutes`, no
  `ci_minutes`); `packaging.md` 2.6 universal test 9.
- Problem: the real survey returns two keys that the prelude's limit table does not have.
- What breaks: once the unknown-key check is built, every survey fails it. Without the check, test 9 tests
  nothing.
- Resolution: add `gate_minutes: 60` and `ci_minutes: 30` to `RUN.limits` in `pipeline.md` 2.11.1.

### P3SEAM-13. The repair review reuses the first loop's step names, so their records collide

- Severity: minor. Slice path: yes (`story.js`, step records).
- Locations: `pipeline.md` 2.11.2 (`review()` is called for the main loop and again after a delivery repair;
  both name steps `review-${l}-p${pass}`, `fix-${who}-p${pass}` and `arbitrate`, with `pass` from 1);
  `state.md` 2.8, Step record path (`<workflow>/<subject>/<name>.<attempt>.json`); 2.10.3 (the verdict is
  computed "from the stored passes of any run").
- Problem: within one run the scoped re-review writes the same keys as the converged loop. Nothing says
  whether `store_step` overwrites a record or refuses.
- What breaks: the verdict that delivery recomputes reads a mixture of the two loops. It either loses the
  converged pass or counts the scoped pass, which had a different item list, as the full review.
- Resolution: prefix the scoped loop's step names with the delivery round (`repair<n>-review-...`). State in
  `state.md` 2.8 that a step record is never overwritten: a second write to an existing key is refused, so a
  collision fails in the harness.

### P3SEAM-14. The arbiter is never given the builder's dispute notes

- Severity: minor. Slice path: yes (`story.js` arbitration).
- Locations: `agents-skills.md` 2.6, architect row ("for arbitration, the open findings and the builder's
  dispute notes"); `pipeline.md` 2.7 loop step 1 ("The fixer's free text stays in the step record for the
  arbiter"), 2.10.8 Fix data ("The `note` is kept in the step record for the arbiter only"); `converge()` and
  `story.js` `arbitrate(open)` (only the findings are passed); every row denies `runs` (`hooks.md` 2.7).
- Problem:
  - `converge()` drops the `note` of each resolution.
  - The arbitration prompt carries only the open findings.
  - The arbiter cannot read the step record that holds the notes.
- What breaks: D18. The arbiter may dismiss only a finding the builder disputed, but it rules without the
  builder's evidence.
- Resolution: `converge()` keeps the notes of disputed resolutions by key and passes them to
  `c.arbitrate(open, notes)`. The scripts put them inline in the arbitration prompt only. Universal test 8
  already keeps them out of reviewer prompts.

### P3SEAM-15. What a re-freeze record lists is not stated, yet two rules read the frozen set from it

- Severity: minor. Slice path: yes (G2 `--refreeze`, `tests-first`).
- Locations: `state.md` 2.16, `gate red` ("a re-freeze record with the new hashes of frozen tests the
  test-writer changed"), 2.8 (evidence lookup returns "the latest passing record, or re-freeze record");
  `hooks.md` 2.11.4 ("a test file that the red record lists as frozen"); `pipeline.md` 2.9.1 (the green gate
  compares "against the latest red or re-freeze record"); `story.js` `fix()` (gates on the entry named
  `red`).
- Problem:
  - If the re-freeze record lists only the changed tests, it becomes the latest record and the other
    frozen tests stop being frozen, for `tests-first` and for the green gate alike.
  - Tests the test-writer adds in a coverage fix are frozen by neither record.
  - The gate name in a re-freeze result (`red` or something else) is unstated, and `fix()` depends on it
    being `red`.
- What breaks: frozen-test protection lapses after the first weak-test fix. A builder who names the result
  `refreeze` sends every test fix to a retry.
- Resolution: in `state.md` 2.16 and 2.8, state that a re-freeze record carries the complete frozen set: the
  earlier identities and hashes, the changed ones with new hashes, and any tests added in the fix pass. Its
  gate result has `gate: "red"`. Cite the rule in `hooks.md` 2.11.4.

### P3SEAM-16. The green gate checks for a "stub marker" that nothing defines

- Severity: minor. Slice path: yes (G2, the `failing-tests` method).
- Locations: `pipeline.md` 2.9.1 ("no stub marker remains"); `state.md` 2.12 (no key); `pipeline.md` 2.2 rule 2
  (no gate names a toolchain); `agents-skills.md` 2.8 `failing-tests`.
- Problem: there is no marker string, configuration key or method instruction for it, and the gate may not
  assume a language.
- What breaks: G2 either cannot build the check or hard-codes a language idiom such as `todo!()` or
  `NotImplementedError`, which breaks the toolchain-neutral rule.
- Resolution: define a fixed marker that the `failing-tests` method writes in every stub (for example the
  token `VSDD-STUB` in a comment). The green gate searches the story's diff for it. State it in
  `pipeline.md` 2.9.1, `state.md` 2.16 (`gate green`) and the method.

### P3SEAM-17. The standard agent frontmatter rejects the key every agent must carry

- Severity: minor. Slice path: yes (A1 agent template and lint).
- Locations: `agents-skills.md` 2.7 ("a six-key frontmatter ... Lint rejects any other key"; "Two more keys
  exist", `omitClaudeMd` and `isolation`; the example `adversary` file has no `omitClaudeMd`); blueprint 4.3
  ("Every agent file sets `omitClaudeMd: true`"); `packaging.md` L-AGENT ("`omitClaudeMd: true` is set").
- Problem: the template and the example contradict the lint and the blueprint.
- What breaks: an agent frame built from the template fails L-AGENT, or a lint built from 2.7 refuses every
  agent file.
- Resolution: in `agents-skills.md` 2.7, list seven keys (adding `omitClaudeMd: true`), plus `isolation:
  worktree` on `spike-runner`, and add the key to the example.

### P3SEAM-18. The spike command forms disagree

- Severity: minor. Slice path: no.
- Locations: `pipeline.md` 2.8, entrance table (`--step <workflow>/<subject>/<name>`; `vsdd spike record
  --json-file <path>`) and 2.9.12 (`vsdd spike record --increment`); `pipeline.md` 2.11.1 `runSpike()` (always
  passes `--step`; `record --id <ledger_id> --json-file`); `state.md` 2.16 (`--step <workflow>/<subject>/<name>.<attempt>`;
  `record --id <ledger id> --json-file <path> [--increment]`); `pipeline.md` 2.8 (the `spike` workflow opens with
  `vsdd spike open --run <id>` and "calls the same `runSpike()` function").
- Problem: the `--step` key lacks the attempt in one place. `--id` is missing in two places. The `spike`
  workflow has no asking step, yet the shared function always passes `--step`, which makes `spike open` look
  for a step record that does not exist.
- What breaks: `/vsdd-factory:spike` is refused at `spike open`, and named-spike increments are written with
  a form the tool does not accept.
- Resolution:
  - Use the `state.md` forms in `pipeline.md` 2.8 and 2.9.12 and blueprint 6.
  - `runSpike()` omits `--step` when it is given no step key.
  - The `spike` workflow calls it without one.

### P3SEAM-19. The `feature` survey must act on arguments its command does not take

- Severity: minor. Slice path: no.
- Locations: blueprint 4.9 and `pipeline.md` 5.1 S2 (`survey <workflow> [--subject] [--repo] [--cycle]
  [--request-uri] [--guidance-uri] [--question-uri]`); `state.md` 2.16 survey row ("with `--amend`");
  `pipeline.md` 2.9.4, survey row (the base is "the release tag for a critical bug"; a request that is "not
  critical ... stays queued"; `amend: true`), 2.3 (`intent`, `severity`).
- Problem: the survey has to know `amend`, `intent` and `severity` before the delta analysis runs, but the
  documented options carry none of them. `--amend` appears in one sentence only.
- What breaks: a critical fix is queued behind the open cycle and cut from the wrong base, and the amendment
  run cannot be started as specified.
- Resolution: add `--amend`, `--intent <value>` and `--severity <value>` to the survey signature in
  blueprint 4.9, `state.md` 2.16 and `pipeline.md` 5.1 S2.

### P3SEAM-20. Every closing command gets `--wait 540`, which only jobs accept

- Severity: minor. Slice path: no (the slice's closing commands are all gates).
- Locations: `pipeline.md` 2.11.1 `brief()` (appends `--wait 540 --json` to every `o.closing`); blueprint 4.9
  (`--wait` only on `gate`); `state.md` 2.16 (`--wait` defined for job commands; `feature route --run <id>`).
- Problem: the architect's closing `vsdd feature route --run <id> --wait 540` uses an option that the
  command does not define.
- What breaks: the delta step's closing command is a usage error, and the route is never written.
- Resolution: state in blueprint 4.9 and `state.md` 2.16 that `--wait <seconds>` is a global option, ignored
  by commands that are not jobs. Alternatively, have `brief()` append it only for job commands.

### P3SEAM-21. `vsdd --version` is main-session-only, but a probe and B2 run it from a workflow step

- Severity: minor. Slice path: no.
- Locations: `hooks.md` 2.7 `[callers]` (`--version` only in `operator`, held only by `main`); blueprint 4.9
  operator row; `packaging.md` 2.6 smoke probe 6 ("`vsdd --version` runs from the main session and from a
  workflow step").
- Problem: `gate-guard` refuses the call for every agent row and for `workflow-subagent` (`other`).
- What breaks: smoke probe 6 fails by design.
- Resolution: move `--version` into the `read` group in `hooks.md` 2.7 and blueprint 4.9. It reveals
  nothing.

### P3SEAM-22. The command table's per-gate callers are not what the hook enforces

- Severity: minor. Slice path: no (`gate-guard` is not in the slice).
- Locations: blueprint 4.9, `gate` row ("The agent whose step ends with it; the planners for `gate spec
  --only` and `gate plan --story`; `clerk`") and the claim that "The hook runner enforces the last column";
  `state.md` 2.16 (per-gate callers: `red` by the test-writer and the clerk, `wave` and `release` by the
  clerk, and so on); `hooks.md` 2.7 (`measure = ["gate"]`, held by every builder, every planner and the
  clerk); `pipeline.md` 2.9.2 (`gate wave --settle` moves stories between waves); `state.md` 2.16 (`gate plan
  --story` admits a story).
- Problem: any builder or planner may run any gate, including the two that change state: `gate wave
  --settle` and `gate plan --story`.
- What breaks: the stated enforcement. An agent off its prompt can change wave membership or admit a story
  to a run.
- Resolution: split `measure` in `[callers]`:
  - `story-gate` (`gate red`, `gate green`, `gate demo`) for builders;
  - `spec-check` (`gate spec --only`) for planners;
  - `admit` (`gate plan --story`) for the story-writer.
  The clerk keeps every gate. Update `hooks.md` 2.7 and blueprint 4.9.

### P3SEAM-23. Write grants for the nested state classes are not defined

- Severity: minor. Slice path: no.
- Locations: `hooks.md` 2.11.2, `write-scopes` check 2 ("P's class is not one the row's write list grants");
  `hooks.md` 2.11.3 and `state.md` 2.4 (the lock treats `brief` as "outside" `spec-product` and `guidance` as
  apart from `spec-arch`); `state.md` 2.4 class table (`brief` written by the product-owner, `guidance` by the
  architect); the architect row `write = ["spec-arch"]` and the product-owner row (`spec-product`, `holdout`).
- Problem: for state paths the rule says "P's class" in the singular. The lock resolves `brief.md` and
  `conventions.md` to their nested class. With that reading, the product-owner may not write the final
  brief and the architect may not write `conventions.md` or `review-rules.md`.
- What breaks: the `brief` skill's final step, the `conventions` method after an ingest, `author
  conventions`, and setup's move of conventions.
- Resolution: in `hooks.md` 2.11.2 check 2, state that a state path's classes are every class whose globs
  match it. The write is granted when the row grants any of them, after check 1 has refused a match that is
  code-only or `none`.

### P3SEAM-24. The lock's liveness fallback relies on a working directory that story steps do not have

- Severity: minor. Slice path: no (`lifecycle-log` is in H9).
- Locations: blueprint 10 item 28 ("kept alive by calls whose working directory lies in a worktree the run
  owns, which the design already does") and 4.13; `state.md` 2.11 and 5 H-2; `hooks.md` 2.11.10; `pipeline.md`
  2.11.1 `step()` (no `isolation` for story steps; only `runSpike()` isolates); `story.js` (steps are told
  "Work only in the worktree", by path).
- Problem: story steps start in the main session's working directory. Reviewers have no shell and never
  change it, and builders change it only if they choose to.
- What breaks: if probe 28 fails, a long review lets the lock go stale after 20 minutes, and a second session
  can take over a live run.
- Resolution: refresh `seen` on any guarded call whose target path or working directory lies in a worktree
  the locked run owns, or whose `vsdd` command carries that run's id. Reword blueprint 4.13 and 10 item 28,
  `state.md` 2.11 and `hooks.md` 2.11.10.

## Checked and found sound

45 items were examined; 26 were sound throughout, and the other 19 are sound except for the findings above.

Sound:
- Plugin layout and Python package paths: blueprint 4.1, `hooks.md` 2.4, `packaging.md` 2.3.
- The launcher chain, the exit-code mapping and the self-guarding `hooks.json` command: `hooks.md` 2.2 and
  2.3, `packaging.md` I3, blueprint 5.4.
- Tiers, the roster and the generated `TIER`, `ROLE` and `OVERRIDE` tables: blueprint 4.2, `agents-skills.md`
  2.3 and 2.12, `packaging.md` 2.5, the prelude.
- The seven product classes and their precedence order: blueprint 4.7, `state.md` 2.4 and 2.12, `hooks.md`
  2.7, `pipeline.md` 2.9.1.
- Path class names across `state.md` 2.4, `agents-skills.md` 2.9 and `hooks.md` `classes.toml`.
- Read walls on `holdout` and `runs`, the code-reviewer's `evidence` and `guidance` exceptions, and the
  scenario-reviewer.
- The whole-set specification lock, the guidance lock and the digest exclusions, beyond R2.
- The decision grammar, the choice table and decision capture through `UserPromptExpansion`.
- Evidence lookup per gate.
- The step record path, and the step key used by `spike open --step` in the prelude and in `state.md`.
- The caller marker path and the command rewrite.
- Gate command options against `story.js`'s closing commands (`red`, `green --since`, `demo`, `deliver`).
- The gate result fields (`unit`, `diff_patch`, `flaky`).
- Delivery states against the delivery loop.
- The workflow result and the `next` object.
- The `HUMAN_ONLY` and `GOVERNED` constants.
- Spike walls, the union of deny sets and the admission record.
- The `vsdd` groups every skill needs from the main row.
- The method table of `pipeline.md` 2.9.13 against the agent method lists (story, wave, spec, feature,
  maintain).
- The event log path and fields.
- Briefing ownership and limits.
- The flush points.
- Build outputs and their sources.
- Nesting, `runStories()` and child budgets.
- Holdout findings and walled keys.
- The route field in `cycle.json`: its writers and readers, beyond R1, R4 and R5.

Sound except for the findings named:
- Roster grants against wall rows: P3SEAM-05.
- Story kinds and exemptions: P3SEAM-08.
- Admission and units: P3SEAM-03, P3SEAM-11.
- The re-freeze: P3SEAM-15.
- Survey and checkpoint options: P3SEAM-10, P3SEAM-19.
- Job re-attach and `--wait`: P3SEAM-20.
- Survey and unit fields: P3SEAM-11.
- Limits: P3SEAM-12.
- Configuration ownership: P3SEAM-06.
- `[callers]` against blueprint 4.9: P3SEAM-21, P3SEAM-22.
- Spike forms: P3SEAM-18.
- Lock liveness: P3SEAM-24.
- Harness tests: P3SEAM-12.
- The conformance fixtures: P3SEAM-02.
- The feature route's input: P3SEAM-01.
- The D4 scope: P3SEAM-05, P3SEAM-09.
- Agent frontmatter: P3SEAM-17.
- Write grants for nested classes: P3SEAM-23.
- Local delivery: P3SEAM-04.
