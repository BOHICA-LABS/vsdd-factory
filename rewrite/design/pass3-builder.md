# Review pass 3: a builder's reading of the thin slice and the build plan

Date: 2026-11-24. Branch `rewrite/v1`. Reviewer prefix `P3BLD-`. Read: `rewrite/STATE.md` in full, the
blueprint, and the five design documents, with the story path read line by line. Not re-reported: R1 to R5 of
`pass2-closeout-3.md`. Line numbers are as of this reading and are given only to help find a passage.

## Verdict

The slice path can be built in outline, but not from the text as it stands: one gap stops it and a dozen more
would make two builders working in parallel build things that do not fit. The blocker is the review verdict
the tool must recompute before it merges. The rule says a pass counts only when the stored agent type owns
the pass's method, but no record stores the method. Nothing says how the tool finds the passes, the items to
account for, the disputes or the rulings among step records. And the scoped review after a failed delivery
overwrites the first review's records, because it reuses their names in the same run. Local delivery, which
the slice uses, merges onto a branch that is checked out in the person's own checkout, and the design both
requires that and forbids touching that checkout. The red and green gates leave open how a test maps to its
file, where the base commit's report comes from, what a stub marker is, and whether a dirty worktree may be
measured. Without the last, green evidence can be recorded for code that was never committed. The machine
records and the tool's own outputs have no schema files, so the store, gate, hook and workflow packages that
the plan builds in parallel have nothing shared to agree on. The fixture the story tier starts from is named
but not defined, and copying an overlay cannot mount a state branch. On the build plan: the slice's package
list in `rewrite/STATE.md` cannot be built in the order section 9 gives. It leaves out B6, C2 and B8, on
which B4a, S1 and A1 depend, and it relies on H7, which needs S4. The sample project sits inside E1, which
depends on nearly every package. S3 is too large for one builder and has dependencies the table does not
show, one of them circular. Five workflow packages miss agent packages that their universal tests need.
Coverage of commands, agents, skills, workflows and rules is complete. Every finding below is a local edit
except P3BLD-01, P3BLD-02 and P3BLD-11, each of which needs a short rule written once.

## What the slice needs from section 9, and in what order

Path: a person types `/vsdd-factory:story S-1.01`. The `story` workflow runs as a standalone run: a clerk
runs the survey, then the tests step (test-writer, red gate), the implement step (implementer, green gate),
the review loop (adversary), delivery (clerk, `vsdd story deliver`, local mode), and a clerk runs the
checkpoint (`pipeline.md` 2.9.1, 2.11.2).

| Package | Depth the slice needs |
|---|---|
| B0, B1, B2, B3, B4a, B4b, H1, H5, A1, W2 | Full. |
| B6 | `roster.toml` for the agents built, and the lints L-AGENT, L-ROSTER, L-METHOD, L-HOOK, L-SH, L-PY, L-MAN. Not in the slice list. B4a needs it (P3BLD-09). |
| B8 | Probes 10, 27, 28 and 31, and 29 if 27 fails. Not in the slice list. A1 depends on it, and H1's caller marker rests on item 27 (P3BLD-09). |
| C1 | Step result, finding, verdict, gate result, workflow result, review, fix and ruling data, and the survey, close and delivery outputs (P3BLD-07). Also the record schemas the design lacks (P3BLD-06). |
| C2 | Full `classes.toml`. The story, contract and epic kinds and the story-kind table in `artifacts.toml`. The rows of `walls.toml` for `main`, `other` and every agent `story.js` names, and `[callers]`. Not in the slice list, but S1 depends on it (P3BLD-09). |
| B5 | The harness, universal tests 1 to 9, 11 and 12, and the story scenarios. |
| S1 | `detect`, mount and `init` (Python detection, local mode), the full key table (an unknown key is an error), and registry validation for the story, contract and epic kinds. Also `-C` and `validate`. |
| S2a | The run, step, evidence, closure, cycle and decision stores (reading only), and the digest. |
| S2b | The lock and reading caller markers. Not the driver. |
| S3 | `survey story` and `checkpoint story`, the preconditions for `story`, the story position, the worktree library, run membership, and enough of the route to return `next` (P3BLD-11, P3BLD-23). |
| G1 | Jobs and the semaphore. Not `env`. |
| G2 | `gate red`, `gate green`, the criterion table of `gate demo`, and the diff patch writer. The baseline comparison is not needed. |
| G3 | The verdict function and grounding, with the story-perimeter conformance fixtures. |
| G4a | The verdict check, the patch-id match, the local-mode merge, worktree removal and the delivery closure. Not push, the pull request, CI, adoption or the `gh` double. |
| H7 | `step-results` only. As packaged, H7 also carries `decisions` and depends on S4 (P3BLD-09). |
| A4, A6a | The test-writer (`failing-tests`, `test-fixes`), the implementer (`tdd`, `fix-findings`) and the adversary (`review-story-diff`). Also the other agents `story.js` names (P3BLD-10). |
| E0 (new) | The sample project, its JUnit runner, the `story-ready` overlay and a loader that mounts it (P3BLD-08). Today these sit in E1, which depends on H1 to H9, W1 and A6b. |

Order. Section 9 runs B0 to B3, B6, B8, R0, C1 and C2 first, then B4a and B5, then the S, G, H and A packages
in parallel. The slice's layer 1 (B0 to B3, C1, B4a, B5) skips B6, which B4a needs. Its layer 2 skips C2, which
S1 needs, and B8, which A1 needs. With P3BLD-09, P3BLD-10 and P3BLD-11 applied, the slice can be built in this
order: B0, B1, B2, B3, B6, C1, C2, B8 (the probes listed), B4a, B5, E0, S1, S2a, S2b, G1, G2, G3, S3a, S3b, G4a,
H1, H5, H7a, B4b, A1, A4, A6a with the agent files `story.js` names, and then W2.

---

## P3BLD-01. The tool cannot recompute the review verdict from what is stored

Severity: blocker. On the slice path: yes.

Where:
- `hooks.md` 2.11.7, item 2. A pass counts only when "the stored agent type is the reviewer that owns the
  pass's method (`review-story-diff` from the adversary, ...)".
- `pipeline.md` 2.10.3, last sentence. `pipeline.md` 2.9.1, review row, last column.
- Blueprint 4.10, Verdict row. Blueprint 5.4, `step-results` bullet.
- `state.md` 2.8: the Step record row (the path is `<name>.<attempt>.json`, and the record holds the result,
  the agent type and the head), and the paragraph on `vsdd.state.verdict(project, subject)`.
- `packaging.md` 2.6, layer 2b, the fixture "A reviewer pass stored with a builder's agent type".
- `pipeline.md` 2.11.2: `review()` names passes `review-${l}-p${pass}` and fixes `fix-${who}-p${pass}`, and the
  delivery loop runs `review(d.reviewed_sha, 2, d.changed_files, 0)` again in the same run.

The problem.
1. The step identity (`run`, `workflow`, `name`, `subject`, `attempt`) and the step record carry no method.
   So the tool cannot apply the very test the design gives it: "the reviewer that owns the pass's method".
2. Nothing tells the tool how to rebuild `converge()`'s inputs from step records, namely:
   - which records are review passes, and their order and pass numbers;
   - which items each pass had to account for (the story's criteria for the first loop, the changed files for
     a repair loop);
   - which findings the builder disputed (the fix records);
   - the arbiter's rulings, the waivers, and the cap.
   The only link between the script's step names and the tool is a naming pattern that appears only in the
   script.
3. The scoped review after a failed delivery starts again at pass 1. It writes `review-adversary-p1.1` and
   `fix-implementer-p1.1` under the same run, workflow and subject. That overwrites the records of the first
   loop that the tool needs.

What breaks. G3 and W2 are built by different builders. If G3 counts nothing, `story deliver` refuses every
merge (`refused`). If G3 guesses a naming rule, the guess has to match the script exactly. After any repair
round, the recomputed verdict is built from records that were overwritten.

Proposed resolution.
1. Add `method` (required) and `loop` (for example `review` or `repair-<n>`) to the step identity. Do it in
   `brief()`'s "Return step = ..." line, in the `step` object of 2.10.1 and of `stepSchema()` in 2.11.1, and in
   what the hook stores (`hooks.md` 2.11.7 item 4). The agent type stays the hook's fact, so writing the method
   in the identity cannot forge a pass.
2. In `story.js`, pass the loop to `review()`. Name steps `review-<lens>-<loop>-p<k>` and `fix-<who>-<loop>-p<k>`,
   so that no two steps of one run share a name.
3. Add one paragraph of verdict inputs to `state.md` 2.8:
   - A pass is the set of done records of the unit whose `method` is a review method of the perimeter and
     whose stored agent type owns that method (from the roster's method table). Passes are grouped by loop and
     pass number, and the latest loop wins.
   - The items to account for are the story's criteria for the `review` loop, and the delivery record's
     `changed_files` for a repair loop.
   - Disputes come from the fix records' `resolutions`, rulings from the `arbitrate` record, waivers from
     settled `finding` decisions, and the cap from `convergence.passes`.
4. G3 writes fixtures for each of these inputs. P3BLD-27 is a smaller alternative.

## P3BLD-02. Local delivery merges onto a branch checked out in the person's checkout

Severity: major. On the slice path: yes.

Where:
- `pipeline.md` 2.9.1, the `vsdd story deliver` bullet: "With `delivery.mode = "local"` (no remote) it merges
  locally".
- `state.md` 2.16, `vsdd story deliver` row. Blueprint 4.12, `delivery.mode`. Blueprint 9.2, G4a ("local mode").
- These contradict:
  - `state.md` 1 ("the person's own checkout is never switched, pulled or committed to") and the 2.3 table,
    row "The person's checkout";
  - `pipeline.md` 3.6 ("Nothing in 1.0 pulls or merges in the person's checkout").
- `hooks.md` 2.11.4 says that in local mode "the integration branch is the product branch", that is, it is
  checked out in the main checkout.

The problem. In local mode the integration branch is normally checked out in the person's checkout. A merge
done there touches that checkout. A merge done elsewhere has to move a branch that is checked out:
- `git fetch .` and `git push .` refuse to;
- `git update-ref` does it, but leaves the person's index and working tree out of step with the branch, so
  `git status` shows the merged change reversed.
The design says none of this, and G4a would have to choose.

What breaks. The first local merge either edits the person's working tree (and fails or mixes changes if the
tree is dirty), or leaves the checkout silently stale. The next story is cut from the moved branch, but the
person and the main session see the old tree.

Proposed resolution. State the mechanism once, in `pipeline.md` 2.9.1 and `state.md` 2.16, and refer to it from
G4a and from `state.md` 1 and 2.3:
1. The tool makes the merge commit in the integration worktree, `.worktrees/_integration/`, detached at the
   integration head.
2. It moves the branch with `git update-ref refs/heads/<integration> <new> <old>`, which fails if the branch
   moved in the meantime.
3. If that branch is checked out in the person's checkout and the checkout is clean, the tool runs
   `git -C <root> merge --ff-only` there. Name this as the one stated exception to "never touched", and
   announce it in the next briefing.
4. If that checkout is dirty, the tool merges nothing and returns `awaiting_merge`, with the line "commit or
   stash your changes, then run the same command".

## P3BLD-03. The script and the tool disagree about an ungrounded finding

Severity: major. On the slice path: yes.

Where:
- `pipeline.md` 2.7, "Blocking finding", condition 4. It says the script applies conditions 1 to 3 and the
  hook or tool applies the fourth.
- `pipeline.md` 2.10.2, the note after the schema. `judge()` in 2.11.1.
- `hooks.md` 2.11.7, item 3. Blueprint 5.4, the `step-results` bullet.

The problem. On the second return of a review that still cites a bad quote, the hook stores the finding marked
`grounded: false`, and so the tool treats it as advisory. The copy the script receives is the agent's own,
with no mark, so `judge()` treats it as blocking. The two verdicts then differ.

What breaks.
- The script runs fix rounds on a finding the tool ignores. This uses Opus passes and can reach the cap, and
  the story then returns `blocked`.
- The checkpoint maps the stored records and finds a converged verdict, so it opens no `finding` decision. The
  story is neither delivered nor waiting on a person until the next run notices the converged verdict.
- The conformance fixtures cannot catch this, because grounding exists only in Python.

Proposed resolution.
- On the second return, `step-results` returns `updatedInput` for the `StructuredOutput` call, with each
  still-ungrounded finding marked `grounded: false`.
- `judge()` and its Python copy treat `grounded === false` as not hard. Add one conformance fixture.
- Add "`updatedInput` is honoured for `StructuredOutput`" to probe 27. If it is not honoured, the fallback is to
  block the second return as well, with the instruction to drop the finding or lower it to `low`.
- State this in `hooks.md` 2.11.7 item 3 and in `pipeline.md` 2.10.2.

## P3BLD-04. The red, green and demo gates cannot be built from their pass conditions

Severity: major. On the slice path: yes.

Where:
- `pipeline.md` 2.9.1, Details, bullets 1, 3 and 4.
- `state.md` 2.16, the `gate red`, `gate green` and `gate demo` rows.
- `hooks.md` 2.11.4: the frozen test files that `tests-first` reads.
- Blueprint 3.3, steps 5, 6 and 8. Blueprint 9.2, G2.

The problem. Six facts the gates and the hook depend on are not defined.
1. Which file a JUnit test case is in. The `file` attribute is optional in JUnit, yet "a test counts when its
   file is in the `tests` class", and frozen tests are recorded by file hash.
2. Where "the report of the base commit" comes from. New tests are "the identities missing" from it, but no
   step produces it.
3. How a test that is "green by design", or a wiring test, is marked as exempt.
4. What a "stub marker" is.
5. What "fails on an assertion and not on a build error" means for a JUnit report. The likely reading is
   `<failure>` against `<error>`. Under that reading, the natural Python stub (`raise NotImplementedError`) is
   reported by `unittest` as an `<error>`, so the slice's sample story would fail the red gate twice and be
   blocked.
6. How the tool links a criterion to its tests when it builds the criterion table for a story with no demo.

What breaks. G2 and H5 would each invent an answer to these, and two inventions need not agree. The slice
itself stops at the red gate if the stub question is answered the obvious way.

Proposed resolution. Add a short "gate inputs" list to `pipeline.md` 2.9.1, mirrored in `state.md` 2.16, and
have the `failing-tests` method carry the agent-side half:
1. Test identity is the JUnit `classname` plus `name`. The file is the `file` attribute, or else the
   `classname` mapped through the `tests` globs. `vsdd init` and `vsdd doctor` warn when the runner gives
   neither.
2. The red gate measures the base once, in a temporary detached worktree, and keeps that report as evidence
   keyed by the base commit.
3. A failure is `<failure>`. An `<error>` whose exception type is in a short constant list (for example
   `NotImplementedError`, `todo!`, `unimplemented!`) also counts as a failure of a stub. Any other error counts
   as broken, and the gate fails.
4. A stub marker is the fixed string `VSDD-STUB`, which the method tells stubs to carry. The green gate fails
   while the string remains in a file of class `source` in the story's diff.
5. A test is exempt when its name contains `wiring` or `green_by_design` (a constant).
6. Each test name contains its criterion, `ac<N>` or `AC-<N>`. The red record keeps the map from criterion to
   tests, and `gate demo` builds the table from that map.

## P3BLD-05. A gate measures a commit in a worktree that may hold uncommitted changes

Severity: major. On the slice path: yes.

Where:
- `pipeline.md` 2.9.1, Details. `state.md` 2.16, the measuring commands ("run from the root of the worktree
  under test", `state.md` 2.12 `[commands]`).
- `state.md` 2.8, Job record row: the job is keyed by command, subject and commit.
- Blueprint 4.13, Evidence lookup.

The problem. The evidence is bound to the head commit, but the configured commands run in the worktree as it
stands. No rule says what happens when the worktree has uncommitted changes.

What breaks.
- An implementer whose passing fix is still uncommitted gets green evidence for the head commit. Delivery then
  merges the committed head, which does not pass. This defeats "a fabricated result cannot merge code".
- The opposite case also goes wrong. An agent commits nothing, edits, and runs the gate again. The job for the
  same commit is found and re-attached, and the old failing result is returned.

Proposed resolution. State in `pipeline.md` 2.9.1 and `state.md` 2.16 that every story gate refuses a dirty
worktree. Untracked files under the report path and ignored files do not count. The refusal returns `passed:
false` with the failure "uncommitted changes in <worktree>: commit them, then run the gate again". Add a G2
fixture for it.

## P3BLD-06. The machine records have no schema files

Severity: major. On the slice path: yes.

Where:
- `state.md` 2.8, opening sentence: "Their field lists are the pipeline design's schemas in
  `<plugin>/schemas/`".
- `pipeline.md` 2.10 holds result schemas only.
- Blueprint 4.10 lists only result schemas. Blueprint 9.1, C1.
- `hooks.md` 5, S7: `evidence()` returns "the frozen test files listed in the red record".

The problem. The run, closure, decision, cycle, evidence and job records are described in prose, by a field
list or by one example. None has a schema file, and C1 delivers none. Their shapes are not settled, for
example:
- the shape of a red record's frozen-test list;
- the patch-id field;
- the run record's list of stories.
Yet S2a, S2b, S3, G2, G3, G4a, H5 and H7 all read or write these records, and section 9 builds those packages in
parallel.

What breaks. Packages that are each correct do not fit together. For example, H5 reads `frozen[].path` while G2
writes `tests[].file`.

Proposed resolution. C1 also writes `run`, `closure`, `decision`, `cycle` and `evidence` schemas, taken from the
field lists of `state.md` 2.8. The evidence schema covers the red record's `frozen` entries (`id`, `file`,
`sha256`), `patch_id`, `sha` and `passed`. `state.md` 2.8 then names those files instead of the pipeline's.

## P3BLD-07. The tool-output schemas are unspecified, and their error shape conflicts with a stated rule

Severity: major. On the slice path: yes.

Where:
- Blueprint 4.10, Step result row: "The script never reads a field the schema does not require".
- `pipeline.md` 2.10.8, the Survey, Close and Delivery rows, which give field lists but no schema and no
  `required`.
- `pipeline.md` 2.11.1: `relay()`'s fallback `{"ok": false, "error": ...}`.
- `pipeline.md` 2.11.2: the script reads `s.preconditions`, `s.units`, `s.run_id`, `d.state` and `c.next`.

The problem. The clerk returns the tool's JSON as a schema result, and the tool prints a short error object on
failure. If C1 requires every field, as the 4.10 rule implies, the clerk cannot return an error object, or the
relay fallback, without inventing values. If C1 requires only `ok`, the script reads fields that are not
required, against 4.10. Section 2.11.1 leaves these literals out ("left out here").

What breaks. C1 must choose, and either choice breaks one of the two stated texts. A clerk that is forced to
fill required fields on an error path may invent a `run_id` or a `state`.

Proposed resolution.
- State in 2.10.8 that every tool output prints every field of its schema, with empty values on an error, so
  that C1 can require them all.
- Give `relay()` a fallback built from the schema: `{...emptyOf(schema), ok: false, error}`.
- Write the survey, close and delivery schemas as JSON in 2.10, as the other schemas are.

## P3BLD-08. The `story-ready` fixture and the way an overlay becomes mounted state are undefined

Severity: major. On the slice path: yes.

Where:
- `packaging.md` 2.6, layer 4. "copied on top" (around line 715), the four fixture shapes (line 728), and the
  story tier "From the `story-ready` fixture" (line 738).
- Blueprint 9.6, E1 and E2.
- `pipeline.md` 2.5, `story` preconditions. `state.md` 2.3 (Mounting) and 2.12 (detection).

The problem.
1. The fixture the slice starts from is named once and defined nowhere. It is not among the four shapes.
2. A standalone story needs:
   - a cycle;
   - an approved `spec` decision whose digest matches the specification set;
   - a contract and a story with its criteria;
   - an admission, meaning the story sits in an open wave of a closed plan (and "open wave" has no fixture-level
     definition);
   - a `factory.toml` with the test commands. `vsdd init` cannot detect the sample's own runner, which is not
     among nextest, `pytest --junitxml`, jest-junit and go-junit-report.
3. `.factory/` must be a worktree on the `factory-artifacts` branch, which copying files cannot produce. Copied
   files give the condition `partial`, in which state writes are blocked.
4. The sample has a local bare origin, which `vsdd init` reads as `delivery.mode = "pr"`. The slice uses local
   mode.

What breaks. E2, and the slice's last step, cannot start without each builder inventing the fixture and its
loading.

Proposed resolution.
1. Define the `story-ready` overlay in `packaging.md` 2.6:
   - `factory.toml` with `test`, `test_report` and `paths`;
   - `cycles/<c>/cycle.json`, with route `standard`;
   - `cycles/<c>/decisions/spec.json`, approved, with its digest;
   - `cycles/<c>/units/plan.json`, closed, with `waves = {W-01: [S-1.01]}` and no `wave.W-01.json`;
   - one contract, one epic and `S-1.01.md`.
2. Add a loader, `tests/e2e/fixtures/load.py`. It runs `vsdd init`, copies the overlay's `.factory/` files onto
   the state branch, computes the digest with `vsdd.state.digest`, and commits. With `--local`, it removes the
   origin.
3. Put the sample, its runner, the overlays and the loader in a new early package, E0, that depends on B3 and S1
   (P3BLD-09).

## P3BLD-09. The slice's package list cannot be built in the order section 9 gives

Severity: major. On the slice path: yes.

Where:
- `rewrite/STATE.md`, the plan items for layers 1 and 2 (around lines 74 to 82 and 99 to 103).
- Blueprint 9.1: B4a depends on B6; B8 "runs early"; H1 depends on B3 only.
- Blueprint 9.2: S1 depends on C2.
- Blueprint 9.3: H7 delivers `decisions` and `step-results` and depends on S4.
- Blueprint 9.4: A1 depends on B8.
- Blueprint 9.6: E1, which holds the sample project, depends on H1 to H9, S3, W1, A4, A6b and G2.

The problem.
1. Layer 1 builds B4a without B6, which holds `roster.toml`, the source of `TIER`, `ROLE` and `OVERRIDE`.
2. Layer 2 builds S1 without C2, A1 without B8, and H7 without S4.
3. H1's caller marker and the survey's hooks-alive check rest on probe 27 (`updatedInput` in a workflow step),
   with probe 29 as the fallback. Neither is proven, and H1 does not depend on B8.
4. Probe 10 (an unresolved MCP tool name in `tools` is dropped silently) decides whether the test-writer and
   implementer start at all. They list DOCS and STORYBOOK tools that a `--strict-mcp-config` run will not have.
5. Probe 31 decides whether `/vsdd-factory:story S-1.01` passes its text.
6. The sample, its runner and the fixtures exist only inside E1.

What breaks.
- Layer 1's B4a has no roster to generate from.
- Layer 2 either pulls in S4 (decisions) or builds half a package with no row for it.
- If probe 27 fails, the survey refuses every run and the slice cannot start.

Proposed resolution.
- Add B6 (the roster and the lints named in the slice table) and C2 to layer 1.
- Run B8 for probes 10, 27, 28 and 31 (and 29 if 27 fails) before H1, and make H1 depend on B8 in 9.3.
- Split H7 into H7a `step-results` (depends on H1, S2a and S3, the last for `unit_tree`) and H7b `decisions`
  (depends on H1 and S4).
- Add E0 (P3BLD-08), and make E1 depend on it.
- Record the slice in section 9 as a milestone: the table at the top of this report, with the depths given.

## P3BLD-10. Workflow packages leave out the agents their universal tests need, and a partial roster fails the lints

Severity: major. On the slice path: yes.

Where:
- Blueprint 9.5:
  - W2 depends on A4, A5 and A6a, but not on A2b;
  - W3 does not depend on A2b;
  - W7 depends on W2, G6b, G7 and A3, but not on A2b or A6b;
  - W11 does not depend on A6b;
  - W12 does not depend on A6b.
- `pipeline.md` 2.11.2: `step('architect', 'arbitrate', ...)`, the security-reviewer lens, and `KIND`, which
  names devops, verifier and technical-writer. `pipeline.md` 2.9.13, rows for wave, verify, maintain and multi.
- `packaging.md` 2.6: universal tests 3 ("every agentType exists") and 6 ("the step's agent owns that method"),
  and lints L-ROSTER and L-METHOD.

The problem.
- `story.js` names eight agents and about fifteen methods. Its package has no dependency on the architect
  (`arbitrate`), so W2's universal tests 3 and 6 cannot pass when W2 is built. The same gap holds for W3 and W7
  (architect), and for W7, W11 and W12 (holdout-evaluator, ui-reviewer, scenario-reviewer).
- The slice's agent list (clerk, test-writer, implementer, adversary) cannot run the full `story.js`, cannot
  pass L-ROSTER against the 20-agent roster, and reaches an agent that does not exist the first time the review
  hits its cap.

What breaks. W2 cannot be accepted. The slice either edits `story.js` away from the design or fails its own
tests.

Proposed resolution.
- Add A2b to W2 and W3, A2b and A6b to W7, and A6b to W11 and W12.
- State in 9.4 that `roster.toml` lists only the agents built so far, and grows with the packages.
- State that the slice ships every agent file `story.js` names, each with only the methods `story.js` uses:
  - the architect (`arbitrate`);
  - the security-reviewer (`security-review`);
  - devops (`repo-setup`), the verifier (`proofs`), the technical-writer (`docs`) and the demo-recorder
    (`demo`).
  Each is a file of at most 60 lines.

## P3BLD-11. S3 is too large, and its dependencies are wrong (one of them circular); a story's position is undefined

Severity: major. On the slice path: yes.

Where:
- Blueprint 9.2, S3 row: the done test, the preconditions, the route with a fixture per row, unit positions,
  the worktree library, admission, decisions opened, cycles opened and closed, the standing cycle, and the
  toolchain write-back. It depends on S2a, S2b and C1.
- `pipeline.md` 2.5: "the survey measures the gate again when only the evidence record is missing". `state.md`
  2.8, done-test bullet 2.
- `state.md` 2.8: the checkpoint writes the verdict into the closure record. Blueprint 4.13.
- Blueprint 9.2: G4a and G5 depend on S3.
- `state.md` 2.6: "Its position comes from evidence", which is the only text about how a position is found.

The problem.
1. One builder would write the survey and checkpoint semantics of all twelve workflows, the route of eighteen
   rows, cycle management and admission.
2. The survey and checkpoint call code from packages that are not listed as dependencies:
   - G2's diff patch writer and red record, for positions and patches;
   - G3's verdict function, for the `reviewed` position and for closure verdicts;
   - G5's and G6a's gates, to measure missing evidence again.
   G5 itself depends on S3, which makes a cycle.
3. `story.js` skips stages by `u.position`, but no text defines how the survey computes `ready`, `red`, `green`,
   `reviewed`, `evidenced` or `delivered`.

What breaks. S3 cannot be finished or reviewed in one package. The builders of S3, G2 and G3 would each invent
the position rules.

Proposed resolution.
1. Split S3 into two packages:
   - S3a: run and unit records, the preconditions, the route with its fixtures, the worktree library,
     admission, cycles and the standing cycle. It depends on S2a, S2b and C1.
   - S3b: positions, the second measurement of missing evidence, and the checkpoint's mapping to closures and
     decisions. It depends on S3a, G2 and G3, and calls gates through a gate registry that S2a defines and the
     G packages fill.
2. Make G5 depend on S3a, and G4a on S3b.
3. Define a story's position in `pipeline.md` 2.9.1:
   - `red`: a passing red record exists in base..head;
   - `green`: green evidence exists for the head, or for a commit with the same patch-id;
   - `reviewed`: the tool's verdict has converged on a pass that covers the head's patch-id;
   - `evidenced`: demo evidence exists for the head, or no demo is required;
   - `delivered`: the delivery closure exists.

## P3BLD-12. Bash calls are located by the payload's working directory, which for a workflow step is the main checkout

Severity: major. On the slice path: yes.

Where:
- `hooks.md` 2.5, Ctx: the `story` row ("the worktree that contains `target` or `cwd`") and the `cwd` row.
- `hooks.md` 2.11.4, `merge-guard`: "while the integration or release branch is checked out".
- `hooks.md` 2.11.10, `lifecycle-log` ("from a working directory inside a worktree that the locked run owns").
- Blueprint 4.13, Liveness. `state.md` 2.11, liveness bullet. Blueprint 10, item 28, fallback column.

The problem. A workflow step that is not isolated starts every Bash call in the project root. The agent's shell
working directory is reset between calls, so a builder runs `cd <worktree> && git commit` or
`git -C <worktree> commit`. The payload's `cwd` is therefore the main checkout. In local mode the main checkout
has the integration branch checked out. Read literally, `merge-guard` then blocks every builder commit. In the
same way, "a working directory inside a worktree the run owns" is never true for a step, so the fallback
offered for probe 28 never refreshes the lock.

What breaks.
- In the slice (local mode), the test-writer cannot commit. The red gate then measures the base commit and
  fails twice.
- If probe 28 fails, a wave's lock goes stale after 20 minutes.

Proposed resolution.
- Define in `hooks.md` 2.5 the effective directory of a Bash call: the last `cd` target or `git -C` path in the
  parsed command, or else `cwd`. `ctx.story`, `merge-guard`, `auto-allow`'s "inside those worktrees" and
  liveness all use it, and liveness also uses `target` for file tools.
- Add two cases:
  - `cd <worktree> && git commit` while the main checkout is on the integration branch: allowed;
  - `git commit` in the main checkout: blocked.

## P3BLD-13. The implementer's `test_defects` is said to reach the test-writer, but no step routes it

Severity: major. On the slice path: yes.

Where:
- `agents-skills.md` 2.9, "Two consequences" (around line 625): "it reports the test under `test_defects` and
  the workflow sends it to `test-writer`".
- `agents-skills.md` 2.6, the implementer row.
- `pipeline.md` 2.11.2, the implement stage. It gives no data schema, reads no `test_defects`, and has no route
  to the test-writer before review.

The problem. When a frozen test is wrong, the implementer cannot change it, and the green gate cannot pass. The
script retries once and then blocks with kind `gate`. Nothing passes the defect to the test-writer, because the
review loop that could do so only starts after green.

What breaks. A wrong frozen test stalls the story. It turns into a `blocked` decision with the reason `stuck`
after two runs, which needs a person for something the design says the workflow handles.

Proposed resolution.
- Give the implement step a data schema that requires `test_defects`, each entry with a `key`, a `path` and a
  `reason`.
- In `story.js`, when the implement result is not green and `test_defects` is not empty, run one
  `fix-test-writer-implement` step (`test-fixes`, ending with `gate red --refreeze`), then the implement step
  once more.
- State this in `pipeline.md` 2.9.1, implement row, and add a harness scenario for it.

## P3BLD-14. Lock liveness and the release at session end are packaged in H9

Severity: minor. On the slice path: yes.

Where: blueprint 9.3, H9 (`lifecycle-log`, which depends on S2c). Blueprint 4.13, Liveness. `state.md` 2.11.

The problem. The once-a-minute refresh and the release at `SessionEnd` belong to the lock, but they arrive only
with H9, after the status and briefing package. Every workflow built and tried earlier, the slice included,
holds a lock that goes stale after 20 minutes. A story with extra-high-effort reviews can run longer than that,
and a second session may then take the lock over.

Proposed resolution. Move the lock refresh and the session-end release into H1, using S2b's `lock_seen` and
`lock_release`, and keep the agent records and pruning in H9.

## P3BLD-15. Where `review_rules` lives is stated three ways

Severity: minor. On the slice path: yes.

Where:
- `pipeline.md` amendment 6: per unit.
- `pipeline.md` 2.10.8: the Survey row has it in `config`, and the Story unit row does not list it.
- `pipeline.md` 2.11.2: the review prompt reads `u.review_rules`. `state.md` amendment 16.

What breaks. A child story receives only its unit. If the survey follows 2.10.8, every story reviewer is told
"Review rules: none".

Proposed resolution. Add `review_rules` to the Story unit fields in 2.10.8 and in `state.md` amendment 16.

## P3BLD-16. Reviewers are given a test output path taken before the tests ran

Severity: minor. On the slice path: yes.

Where: `pipeline.md` 2.11.2 (`Test output: ${u.test_output}`); 2.10.6 (the gate result has `diff_patch` but no
test output); `state.md` 2.16 `gate green` (it writes the test output).

The problem. In a run that starts from `ready`, `u.test_output` was computed by the survey before any green gate
ran. Each green result replaces `patch`, but nothing replaces the test output path.

Proposed resolution. Add `test_output` to the gate result for `green`, and have `story.js` update it in the same
places it updates `patch`.

## P3BLD-17. A standalone story returns without its checkpoint after a survey that took the lock

Severity: minor. On the slice path: yes.

Where: `pipeline.md` 2.11.2, `if (!s.ok || unmet.length || !s.units.length) return ...`. It contradicts 2.5
("Every return after a successful survey goes through the checkpoint") and blueprint 4.13.

The problem. With `s.ok` true and every precondition met, the lock is taken. The `!s.units.length` branch then
returns without the checkpoint, and the lock stays held until it goes stale.

Proposed resolution. Drop `!s.units.length` from that test. The precondition "the story is admitted" already
guarantees a unit, so a survey whose preconditions pass always returns one.

## P3BLD-18. `checkpoint --subject` is used but not defined

Severity: minor. On the slice path: yes.

Where: `pipeline.md` 2.11.2 and 2.11.3 pass `--subject`. Blueprint 4.9 and `state.md` 2.16 define
`checkpoint <workflow> --run <id> [--outcome failed]`.

Proposed resolution. Remove `--subject` from both skeletons. The run record already holds the subject.

## P3BLD-19. Universal test 7 cannot pass

Severity: minor. On the slice path: yes.

Where: `packaging.md` 2.6, universal test 7. `pipeline.md` 2.11.2 and 2.11.3: a survey that is not ok returns
before the checkpoint, correctly, because no lock was taken.

The problem. With every agent returning `{status: 'done'}`, the survey stub is not `ok`, so no run reaches the
checkpoint.

Proposed resolution. Reword the test: "With the survey stub returning a passing survey and every other agent
returning `{status: 'done'}` and empty data, and again with one agent throwing, the run ends by calling the
checkpoint."

## P3BLD-20. The prelude's `RUN.limits` lacks two names the survey returns

Severity: minor. On the slice path: yes.

Where: `pipeline.md` 2.11.1, `RUN.limits`, which has no `gate_minutes` and no `ci_minutes`. Blueprint 4.12 and
`state.md` 2.12 list both as names under which the survey returns limits. `packaging.md` 2.6, universal test 9
("a limit key the prelude does not know fails the run").

What breaks. Every real survey fails test 9. Or the check is left out, and test 9 then means nothing.

Proposed resolution. Add `gate_minutes: 60, ci_minutes: 30` to `RUN.limits` in the prelude.

## P3BLD-21. The prelude build contradicts its own check and its line cap

Severity: minor. On the slice path: yes.

Where: `packaging.md` 2.5, the table rows "Schema literals" and "The `TIER`, `ROLE` and `OVERRIDE` tables" ("a
marked block inside the prelude"), the `--check` list ("equals `workflow-lib/prelude.js` byte for byte"), and the
constraint "at most 400 lines". Blueprint 4.14.

The problem.
- If the generated blocks are written into the stamped copies, no copy equals `prelude.js`.
- If they are written into `prelude.js`, the source is partly generated, and the text does not say so.
- The full schema literals, plus the roughly 300 lines of functions in 2.11.1, exceed 400 lines.

Proposed resolution. State the order: `build.py` first regenerates the marked sub-blocks inside
`workflow-lib/prelude.js`, then stamps the file into the workflows. The 400-line cap excludes the generated
sub-blocks.

## P3BLD-22. The sample's test command writes into the story worktree and may try to download

Severity: minor. On the slice path: yes.

Where: `packaging.md` 2.6, layer 4 (`uv run python tools/junit_runner.py`, `junit:build/junit.xml`).

The problem. `uv run` inside a project:
- creates `.venv`;
- writes or updates `uv.lock`, which falls in the `manifests` class;
- builds and installs the project if it declares a `[build-system]`, which needs a download, and a download
  failed in the spike's sandbox.
`build/junit.xml` also lands in the worktree. Together these leave the worktree dirty (P3BLD-05), put a lock file
into the story's diff, or fail offline.

Proposed resolution. Make the sample's command `uv run --no-project python tools/junit_runner.py`. Give the
sample a `.gitignore` with `build/` and `.venv/`, and no `[build-system]`.

## P3BLD-23. Run membership for a standalone story, and the meaning of "a live run", are not stated

Severity: minor. On the slice path: yes.

Where:
- `pipeline.md` 2.9.1, "every gate and `vsdd story deliver` check that the story is recorded in the run they are
  given".
- `state.md` 2.8, Run record row: "the stories admitted to it", written by `vsdd survey` and "the admission
  commands".
- `state.md` 2.16, `vsdd survey` row.
- `hooks.md` 2.11.7, item 2 ("only when `step.run` names a live run").

What breaks.
- If the survey does not add the story it returns to the run record, every gate refuses the slice's story.
- If "live" is read as "holds the lock", step records from runs that take no lock (`spike`) are dropped. Those
  records include the stored `needs_spike` record that `spike open` reads.

Proposed resolution.
- State that the survey records every story unit it returns in the run record's `stories`, the same list the
  admission commands append to.
- Define a live run as one whose run record exists and has no checkpoint recorded.

## P3BLD-24. Build items that no package delivers, or that two packages deliver

Severity: minor. On the slice path: yes (through `unit_tree`).

Where:
- `packaging.md` 2.5: "`[reveals]` ... is generated ... by the same build". B4a's row omits that generator, and
  C2 delivers only `walls.toml`.
- `hooks.md` 2.11.7, item 3, uses `unit_tree`, which S3 provides. H7 does not depend on S3.
- Blueprint 9.6, T5, delivers `<plugin>/assets/`, but the methods of A2a and A5 cite those assets, and L-TEMPLATE
  checks them.
- The shared block and the severity table are delivered by both B4a and A1. Agent lint is delivered by both B6
  and A1.

Proposed resolution.
- Give the `[reveals]` generator to B4a.
- Make H7a depend on S3a (P3BLD-09).
- Move `assets/` to the A packages whose methods cite each part: `scaffold/` to A5, `demo/` to A5,
  `design-system/` to A2a.
- Leave the stamped texts in B4a and agent lint in B6. A1 consumes both.

## P3BLD-25. The rewritten `vsdd` command goes through ordinary permission checks, so the static-rule fallback no longer matches

Severity: minor. On the slice path: no.

Where:
- `hooks.md` 2.11.10, caller markers (the command is rewritten to begin `VSDD_CALLER=<key> `).
- Blueprint 10, items 7 and 30, fallback column ("static allow rules").
- `hooks.md` 2.16, the `updatedInput` row.

The problem. The hook documentation says that `updatedInput` with no `permissionDecision` "flows through the
normal permission evaluation", so that evaluation sees the rewritten command. A static rule such as `Bash(vsdd:*)`
does not match a command that begins with an environment assignment. So the fallback for D4 would prompt for
every `vsdd` call that `auto-allow` does not permit.

Proposed resolution.
- Add to probe 27: "and a static allow rule still approves the rewritten command".
- If it does not, `auto-allow` returns `Permit` together with `updatedInput` for every `vsdd` call that the
  caller's row grants. The fallback then needs no static rule for `vsdd`.

## P3BLD-26. Simplification: inject the caller facts into the command instead of writing marker files

Severity: minor. On the slice path: yes.

Where: blueprint 4.7 (Caller marker row), 4.9 ("How the tool knows who is calling"); `hooks.md` 2.11.10;
`state.md` 2.11.

The proposal. The runner already rewrites every `vsdd` command. It could prefix the facts themselves, for
example `VSDD_CALLER_SESSION=… VSDD_CALLER_AGENT=… VSDD_CALLER_TYPE=… VSDD_CALLER_CWD=…`, instead of a key to a
one-time marker file that the tool reads and deletes.

What it keeps.
- Forgery protection is unchanged: `gate-guard` blocks any `VSDD_*` assignment the caller types, and the prefix is
  added only after every rule has judged the command.
- With the hooks off there are no variables, so the survey refuses exactly as it does without a marker.

What it saves. One of the 14 record types, key generation, the pruning of unread markers, and the race in which a
marker is read and deleted. The cost is that the caller facts appear in the command text, and values with spaces
must be quoted.

## P3BLD-27. Simplification: the tool checks the last pass instead of re-running the whole loop

Severity: minor. On the slice path: yes.

Where: blueprint 4.14 (the convergence rule and "the tool's Python copy"); `pipeline.md` 2.10.3; `packaging.md`
2.6, layer 2b.

The proposal. The merge guard needs only one fact: the latest pass, by its owning reviewers, at the head's
patch-id, accounted for every required item, and left no blocking finding. A finding the arbiter dismissed after a
dispute, or a person waived, does not count. Cap counting and dispute bookkeeping steer the loop; they are not
the guard.

Under this proposal, the tool's verdict becomes:
- `judge()` in Python, applied to the last pass;
- minus dismissals, taken from the `arbitrate` record checked against the fix records' disputes;
- minus waivers;
- with `rounds_used` as the count of passes stored for the unit.

What it saves:
- the Python port of `converge()`;
- about half of the conformance fixtures;
- most of the reconstruction problem in P3BLD-01, since only the last pass and its loop must be identified.

What it keeps: every enforcement.

---

## Checked and found sound

52 items checked; 25 found sound and 27 led to the findings above.

1. The launch chain: the exit-code mapping from 10 to 2, uv discovery, the Python floor, the flags
   (`hooks.md` 2.2).
2. The self-guarding `hooks.json` command, and the inert path for projects that are not initialised (`hooks.md`
   2.3 and 2.8).
3. Event wiring: `StructuredOutput` and `Workflow` reach the runner; unknown tools pass; seven events.
4. The rule model, outcome delivery, all selected rules running, and the block message.
5. The five project conditions and `detect()`, which agree across `state.md` 2.3, `hooks.md` 2.8 and blueprint
   5.2.
6. Mounting: the orphan branch, the `.git/info/exclude` lines, `local/` ignored, no `.factory` in story worktrees.
7. Flush: the five points, an empty hooks path, no signing, idempotence.
8. Taking the lock: atomic, after preconditions, scoped to repository and branch, so a standalone story and a
   wave exclude each other. Liveness is apart (P3BLD-12, P3BLD-14).
9. Product path classes and their precedence, one rule shared by `tests-first`, `write-scopes` and the red gate.
10. The grants of `tests-first` (`source`, `source:before-red`, `tests:frozen`), and its red-evidence lookup on
    the branch since its base.
11. The command list of `merge-guard`, the `GOVERNED` constant, and the absence of `pr-merge` (cwd apart,
    P3BLD-12).
12. The `walls.toml` rows for clerk, test-writer, implementer and adversary, against blueprint 4.3 and
    `agents-skills.md` 2.5 and 2.9.
13. The tool lists of the four slice agents. Steps have no Agent tool (proven, item 8).
14. The step identity round trip: `brief()`, then the name and subject check in `step()`, then the hook storing
    the record by identity.
15. `gated()` in single-gate mode for tests and implement: the retry carries the failure text, and `sha` equals
    `head`.
16. Story kinds, which agree across blueprint 4.6, `story.js` `KIND`, `pipeline.md` 2.9.13 and the red-gate
    exemptions.
17. Tier selection: `TIER` and `ROLE` in `step()`, the clerk tier in `relay()`, and the `VSDD_TEST_TIER`
    override.
18. Long commands: detached jobs keyed by command, subject and commit, `--wait 540` with a Bash timeout of
    600000, and the re-run while pending.
19. The delivery data fields `story.js` reads, and its handling of each delivery state.
20. The survey unit fields `story.js` reads, all present apart from `review_rules` (P3BLD-15).
21. Command coverage: every command in 4.9 is delivered by some package in section 9.
22. Coverage of the rest: 20 agents (A1 to A6b), 13 skills (K1 to K3b), 12 workflows (W1 to W12), 17 rules
    (H2 to H9), and 74 packages counted.
23. The one-lens story review loop: reach, claims with no quote and no note, and dismissal by the arbiter only
    after a dispute (outside R1 to R5 and P3BLD-03).
24. Exit codes, the `-C` option, and clerk commands as one segment.
25. The section 10 facts the slice relies on that are proven: items 1, 5, 8, 11 (`omitClaudeMd`) and 15, and
    `agent_type` in a step's payloads.
