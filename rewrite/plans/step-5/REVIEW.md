# Review of the step 5 plan (`rewrite/plans/step-5/agents.md`)

Reviewer: Opus, against the story script (`topics/story-script.md`, `workflows/story.md`, `next/workflow-lib/prelude.js`),
the shipped policy tables (`next/plugins/vsdd-factory/py/vsdd/policy/{walls,classes,artifacts}.toml`), the shipped
roster and lint (`next/tests/lint/roster.toml`, `test_roster_lint.py`, `tools/build.py`), `tables/lint-checks.toml`,
`topics/tool-sets.md`, `topics/research-and-browser-tools.md`, the agent spec files, the step 3 plans, the ledger and
`rewrite/history/design-v1/agents-skills.md`.

## Verdict

Build after changes. No blocker: from these files E2's one story ("add `median()`", a feature story with no demo,
local mode) can reach delivery. Four majors remain. Two of them need decisions outside step 5 before E2's first run:
the spike-runner (PL5-3) and how a shell-less reviewer finds a contract (PL5-2). PL5-1 is a one-line fix that the
slice's only story depends on.

## What was checked and holds

- **Every step has its agent and method.** Every step `story.js` runs maps to an agent and method of the plan under
  the same names: `survey`, `deliver-<n>` and `close` (clerk); `tests` (test-writer, the kind's tests method);
  `implement` and `implement-2` (the kind's implement agent and method); `test-defects` (test-writer, `test-fixes`);
  `review-adversary-*` (`review-story-diff`); `review-security-reviewer-*` (`security-review`); `fix-test-writer-*`
  (`test-fixes`); `fix-implementer-*` (`fix-findings`); `arbitrate-l<n>` (architect, `arbitrate`); `evidence`
  (demo-recorder, `demo`); `repair-<n>` (implementer, `fix-findings`). The one agent the stamped prelude starts that
  the plan does not build is the spike-runner (PL5-3).
- **Data schemas.** Each step's data schema is the one the script passes: `S_IMPLEMENT_DATA` on both implement steps,
  `S_REVIEW_DATA`, `S_FIX_DATA`, `S_RULING_DATA`, and none on `tests`, `test-defects`, `evidence` and `repair`. The
  clerk returns the relayed schema itself, not a step result (`relay()`).
- **Reading of `gated()` and `review()`.** D11 (`head` is the closing gate's `sha`), D12 (one entry per gate,
  because `gated()` takes `all.slice(0,1)`) and D15 (`reviewed_sha` copied from `from...head`) are correct. Without
  them `gated()` (prelude.js 3334-3339) and `review()` (`r.data.reviewed_sha !== head`) fail.
- **Tool lists.** Every tool list in section 3 equals `topics/tool-sets.md`: R is READ, REPORT, SPAWN and RESULT; the
  architect's `Bash` is its VSDD shell (`tool_only = true`). Both MCP spellings are named for DOCS, LOOKUP and
  BROWSER, and only `mcp__storybook__*` for STORYBOOK.
- **Closing commands against the walls.** Every closing command is in a group the agent's row grants: `gate red`
  and `gate red --refreeze` (test-writer: story-gate, refreeze); `gate green`, including `--since` (each builder:
  story-gate); `gate demo` and `env start` (demo-recorder: story-gate, env); survey, story deliver and checkpoint
  (clerk: pipeline). Git is git-commit plus git-read (M2). The reviewers have no shell.
- **What the walls enforce.** The implementer cannot write `tests`. The test-writer writes `source` only before
  red. Devops cannot write `config` and creates source and tests only in a `repo-setup` story. Reviewers write
  `reviews:create` only. The clerk runs `vsdd` only.
- **The three lint and build defects are real.**
  - D6: `build.py` 354-364 compares the tier name `"reviewer"` with the model `"opus"`, so no reviewer is stamped,
    while the lint stamps by tier name and reports drift.
  - D7: `test_roster_lint.py` 207-209 counts stamped lines in the 70. A reviewer with a 40-line body is already
    about 77 lines.
  - D8: `KNOWN_TOOLS` (51-71) holds only bare MCP names.
- **The proven platform facts are respected.** Every agent has an explicit tool list. `Agent` is listed but is
  relied on only for a direct dispatch. No agent sets `isolation`, hooks, MCP servers or a permission mode. The
  model and effort in a file are the defaults for a direct dispatch.
- **Step 3 outputs the methods use.** The methods use the outputs the step 3 plans settle:
  - `vsdd path <id>` prints the bare path without `--json`.
  - `vsdd config --json` gives `{config, hook_problems}`, so M2 reads `config.paths` and `config.commands`.
  - `vsdd env start --subject` gives `{ok, url, port, pid}`.
  - The unit's `acs` are ids (survey D15), and `test_output` is empty on a first run (survey D16), which is why
    D14 makes it optional.

## Findings

### PL5-1 (major) `failing-tests` forbids changing existing source, which E2's story needs

`failing-tests` lists "Not: implement behaviour, change existing source." The wall grant is
`source:before-red` (not `:create`), and the design limits source writes before red to stubs, not to new files.

E2's story adds `median()` to the sample's one existing module (`packaging/sample-project.md`,
`packaging/e2e-story-tier.md`). The stub belongs in that existing file, because that is where the tests import it
from. A test-writer that obeys the line either invents a new module or returns blocked, on the slice's only story.

Resolution: replace the item with "Not: change the behaviour of existing code; before red, source edits are stubs
only (a new symbol's signature, a body that raises the not-implemented error, a `VSDD-STUB` comment), added to the
file where the symbol belongs." Use the same wording in `regression-test` and `equivalence-tests`.

### PL5-2 (major) How a shell-less reviewer finds a contract contradicts the shared block and the plan's own D14

The script passes contract ids only. M1 has the adversary and the security-reviewer Glob `**/<id>.md` under
`<project dir>/.factory/specs`. That builds a path into the state directory, which is exactly what the stamped
shared block forbids ("Do not build a path into the state directory yourself"). It is also the reason D14 gives for
blocking the demo step.

A reviewer that follows its own file returns blocked for a missing input. That is `reviewer_blocked`, and on E2's
path the story ends blocked. The search itself is also unproven: the design keeps `.factory/` excluded so that
search tools skip it (probe item 14, Grep), and a Glob rooted inside the state worktree from a workflow step has not
been probed.

Resolution: use one rule for both cases.
- Preferred: S3 and W2 pass contract paths. The unit resolves `bcs` to paths, or the review prompt names them. Every
  method then reads them as given. Record this with N1 as a new finding.
- Until then: M1 names `<project dir>/.factory/specs` as the one state root a shell-less agent may search, and the
  adversary's and security-reviewer's Inputs say so.
- Either way, add a check, to E1's smoke probes or B8, that a workflow-step Glob finds a contract, before E2 runs.

### PL5-3 (major) The spike-runner is not built, but any step may ask for a spike

The shared block tells every workflow step to return `needs_spike`. `step()` then calls `runSpike()`
(prelude.js 3267-3287), which starts `vsdd-factory:spike-runner`. After step 5 there is no `agents/spike-runner.md`
and no `methods/spike.md`; the agent is package A3.

Starting an unknown agent type either throws, and `story.js`'s catch ends the run `failed`, or it returns nothing.
Nothing gives an inconclusive finding; the step runs again, and after `spikes_per_step` it ends blocked with
`spike_limit`. Either way, one spike request in E2's run keeps the story from reaching `done`. The plan leaves this
as open question N7.

Resolution: decide before the build and write the decision into the plan.
- Either add the spike-runner, with the `spike` method only, to step 5. It is named by the prelude that `story.js`
  stamps, so it is one of the agents the script reaches. Its row, tool sets and block variant already exist.
- Or record that E2's first run counts a spike request as a failure and has no spike scenario.

### PL5-4 (major) The demo step cannot succeed as planned

Not on E2's path (the median story has no `demo` key), but `demo.md` would have to be rebuilt.

1. D14 requires an evidence directory that `story.js` never passes, so every story with `demo` set blocks (N1).
2. Even with a directory, the method does not say how its files reach the state directory. Write-scopes check 4
   blocks every Bash write into `.factory/` (redirect, `tee`, `cp`, `mv`, ...), so "its real output as `AC-<k>.txt`"
   made with a redirect is blocked. No route at all is given for `AC-<k>.png`.
3. M3's "Commit everything" does not fit this step. Demo evidence is state on the state branch, which flush commits
   (`topics/state-commits.md`). The story worktree has nothing to commit, and a demo-recorder that commits under
   `.factory/` goes against the design.

Resolution:
- Mark N1 as blocking for every demo story (W2 or S3 pass the directory, or `vsdd path` resolves it).
- In `demo.md`: run the scripts from the worktree or the temporary directory; write each transcript with the Write
  tool from the captured output, unchanged; name the screenshot route (the browser tool's file name, into the
  directory given) and mark it unproven.
- Replace M3's commit line for this method with "commit nothing; evidence is state". Keep `head` = the gate's `sha`.

### PL5-5 (minor) M3's retry clause collides with `test-fixes`' green run

M3, in every builder's Finish, says "On `passed: false` act on failures, commit and run again". In `test-fixes` a
failing green run is the expected signal: `story.js` uses `red = !pick(r,'green').passed` to send the implementer.
A test-writer that acts on that failure can only change tests, which means undoing the strengthening it just did.
Step (3) ("pass or fail") and the Do not ("weaken") point the other way, but both texts sit in one Finish.

Resolution: M3 applies to the closing command only. `test-fixes`' Finish says the green output is returned as it is
and never acted on.

### PL5-6 (minor) D18 misreads VSA-18

D18 and open question 8 say a conflict repair will usually block because the worktree is reset. VSA-18's settled
outcome (left-for-builder on W2) is that the tool brings the integration head into the story branch before the
repair. The markers are then in the story worktree, and `git add` plus `git commit` (git-commit family) resolve
them. The reset in `story-deliver.md` is of the integration worktree.

Resolution: D18 says a conflict repair depends on G4a carrying out VSA-18. `fix-findings` resolves the markers and
commits, never aborts the merge, and returns blocked (`conflict`) only when no markers exist.

### PL5-7 (minor) "The example changed four ways" undercounts the adversary's changes

The example's text differs from the plan's frame in three places:
- Its Method text ("Your task names one of ... Read `${CLAUDE_PLUGIN_ROOT}`...") is not the common Method text that
  section 2 item 5 and D3 require.
- Its Inputs lack the blocked ending of item 4 and the optional inputs of D14.
- Its Result lacks the template's two closing sentences (item 6).

A builder that follows section 3 literally writes a file that disagrees with section 2.

Resolution: list every change, or say: the example's opening and Rules, adapted as listed, inside the frame of
section 2.

### PL5-8 (minor) AU-S2-22 is handled against its own resolution and against the plan's no-copy rule

Section 2 forbids "a copy of a wall row". AU-S2-22's resolution is to link the row. Yet sections 3 and 6 have the
adversary's read bullet name all four denied classes (`holdout`, `reviews`, `runs`, `outside`), which is the row's
`read_deny` copied.

Resolution: the bullet names what matters to the role ("earlier reviews or holdout scenarios (enforced)") and says
the full wall is the agent's row. Record the outcome as linked, not enumerated.

### PL5-9 (minor) L-KIND is already active in B6's lint

The plan says B6 defers L-METHOD and L-KIND. The shipped `test_roster_lint.py` has `kind_grant_problems` and runs it
in `check_all`. The "hold the L-KIND grants" half of `test_agent_files.py` would be a second copy of that check.

Resolution: drop the grants half. Keep "each kind's agents list the kind's methods".

### PL5-10 (minor) "Each method file is listed by exactly one agent"

L-METHOD requires at least one agent. `tables/methods.toml` gives some methods several owners (`score-ideas`:
product-owner, architect, adversary; `intake`: product-owner, architect), so the test breaks when those methods
ship.

Resolution: each method file is listed by the owners `methods.toml` gives it and by no other agent; or, until B6
ships that table, by at least one agent.

### PL5-11 (minor) D8 still accepts bare MCP names

D8 adds the prefixed forms, but `KNOWN_TOOLS` keeps the bare names (`query-docs`, `browser_click`). A bare name never
resolves, and probe item 10 shows an unresolved name is dropped silently. A file that lists one passes the lint and
loses the tool.

Resolution: an MCP tool is known only in a prefixed form. Refuse bare server tool names, with a fixture.

### PL5-12 (minor) The architect's "whole role" collides with the registered-subcommand test

The architect's spec file names `vsdd feature route` and `vsdd gate spec --only`. Neither is built by steps 3 or 4.
Section 7 item 3 requires every `vsdd` subcommand the 25 files name to be registered.

Resolution: describe the whole role without naming unbuilt subcommands, or limit the check to the commands the
built methods run. Say which in the plan.

### PL5-13 (minor) Fix routing for docs, chore and harness stories

`review-story-diff` sets the owner to `implementer` for every finding that is not about tests, as `story.js` routes.
In a docs, chore or harness story the implementer's row cannot write `docs`, `ci`, `repo-config` or `harness`, so
that story's review loop cannot fix a finding about its own output. Not on E2's path.

Resolution: a new finding for W2: the fix step uses the kind's implement agent and its fix method. The owner rule in
the method then reads "the story's builder".

### PL5-14 (minor) Two parts of the step prompts that no method handles

- The review task carries "Not yet covered: <gaps>", and `review-story-diff` says nothing about it. The section 1
  table also omits it.
- The test-writer's fix task carries "Fix these too where it is cheap: <advisory>", and `test-fixes` says nothing
  about it (`fix-findings` (3) does).

Resolution: in `review-story-diff`, give each not-yet-covered item a coverage entry first. In `test-fixes`, add the
advisory step that `fix-findings` (3) has.

### PL5-15 (minor) Four methods are built that cannot run in the slice

- `equivalence-tests` and `port-translation` need the reference implementation, and `service-clone` needs the
  service's recorded behaviour. Nothing passes either. Under D14, a missing required input means blocked.
- These three and `regression-test` are reached only by kinds the slice never runs.
- `port-translation`'s "the contract wins: blocked, `spec_gap`" contradicts itself.

Resolution: keep them, for L-METHOD reachability, but thin. Each names where its input comes from (a story key, or
a `.reference/` path given) or returns blocked (`environment`). Record the missing inputs with N1. Reword
`port-translation`: "where reference and contract differ, stop: blocked, `spec_gap`".

### PL5-16 (minor) Rules bullets that are neither enforced nor visible, and one that overstates

Section 2 requires each Rules bullet to be enforced or visible in the result:
- "never weakens a test" (test-writer) and "current behaviour only" (technical-writer) are neither.
- "writes only the evidence directory (enforced)" (demo-recorder) overstates. The row grants the whole `demo` class
  and the temporary directory.

Resolution: tie each bullet to the result (for example, the summary names every test changed and why) or move it to
the method's Do not. The demo bullet becomes "writes only demo evidence and temporary files (enforced)".

### PL5-17 (minor) Details missing from the blocked paths

- D16 names no `blocked.kind`.
- M4 does not say what `head` is for a builder that stops before its gate.
- `test-defects` assumes `gate red --refreeze` passes when no frozen test changed. G2 is not planned yet.

Resolution: kind `other` for an unlisted method or a missing method file. `head` is the worktree's current commit,
or empty, when no gate ran. Add the no-change re-freeze to the G2 open question, next to the gate-name question.

### PL5-18 (minor) Bookkeeping

- N10 is already the ledger's L2-c2-1 (W2, open): the unit must carry `tests_agent`, `tests_method`, `impl_agent`
  and `impl_method`.
- Narrowing universal test 3 to `toy-steps.js` and `workflows/` relaxes an existing check. Section 8 says the
  builder "never weakens a check".

Resolution: cite L2-c2-1 instead of a new N10. State the test-3 narrowing as temporary, with its restore condition
(every agent a toy calls has a file), and keep the roster check over all toys.

## Dependencies the plan states, confirmed

- **C2's policy tables** exist on disk under `py/vsdd/policy/`. B6's lint exists, and its outcome is recorded in
  `rewrite/findings/outcomes/b6.json`. The build still waits for `slice-layer-2a` to report finished
  (`rewrite/STATE.md`).
- **The plan edits B4a's and B6's files** (D5 to D8, open question 3). These are small, tested and justified, but
  the orchestrator must confirm that `slice-layer-2a` has finished, so that two workflows do not edit
  `test_roster_lint.py`, `build.py` or `universal.test.mjs` at once.
- **G2 is not planned yet.** The plan assumes `gate red --refreeze` returns a result whose `gate` is `red` (which
  `gated(..., 'red', ...)` needs) and that a re-freeze with nothing changed passes (PL5-17).
