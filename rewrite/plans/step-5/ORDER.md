# Step 5: the one unit, what it owns, and the corrections that hold over its plan

Step 5 of the thin slice is the ten agent files and fifteen method files the story script dispatches
(`rewrite/spec/packaging/build-plan.md`, "The thin slice"). It is one unit, built from `agents.md` in
this directory by one run of `rewrite/workflows/slice-build-unit.js` with
`args: {step: 5, key: "agents", title, packages, concurrent, notes}`. The plan was corrected against
`REVIEW.md` (`RESOLUTIONS.md` says how). Things learned after that are in section 3 of this file; where
this file and the plan differ, this file holds.

Paths are under `next/`; `P` is `plugins/vsdd-factory`.

## 1. When it is built, and beside what

After step 4's `verdicts`, `gates`, `rules` and `survey-2` have reported with nothing blocker or major
open. It may run beside step 4's last unit, `delivery`, which owns `P/py/vsdd/delivery/` and its own
tests under `tests/unit/` and registers the command `vsdd story deliver`. Two things join them:

- The test of plan section 7 item 3, that every `vsdd` subcommand the 25 files name is in
  `vsdd.cli.dispatch.commands()`, needs `story deliver` to be registered. Until the delivery unit has
  landed its command module that one name fails. It is not worked around: run the test again at the
  end, and if the name is still missing list it as a foreign failure.
- This unit changes `tools/build.py` and `tools/check.py`, which the delivery builder runs. Make each
  change of those two files as one write that leaves the file working, never in halves.

## 2. Files the unit owns

As plan section 8 ("May write") gives them: the ten files under `P/agents/`; the fifteen under
`P/methods/`; `workflow-lib/finding-key.md` and `workflow-lib/agent-shared-clerk.md`; the D5 and D6
changes in `tools/build.py` and `tools/check.py`; the D7 and D8 changes in
`tests/lint/test_roster_lint.py`; the new cases in `tests/lint/test_build.py`;
`tests/lint/test_agent_files.py`; `tests/workflows/toys/toy-steps.js`; and tests 3 and 6 of
`tests/workflows/universal.test.mjs`. Nothing else under `next/`: not `workflow-lib/prelude.js`, not the
text of `agent-shared.md` or `severity.md`, not `tests/lint/roster.toml`, not the policy tables, not the
other toys, not the `storyUnit` literal of `universal.test.mjs` (step 4's `survey-2` wrote it).

## 3. Corrections that hold over the plan

Each names the plan section it changes. The builder applies them as it writes; it does not edit the plan.

1. **Three more agents fix findings** (`PL6-1`, `S5P-N10`; blocks the story workflow). Section 3: the
   technical-writer, devops and verifier each list `fix-findings` beside their own method. Section 7
   item 3: the method table of `test_agent_files.py` says so. Section 4: the head of `fix-findings`
   becomes "(`implement`, fix; `fix-<the kind's implement agent>-*`; `repair-<n>`)", and the method
   gains the sentence "For a kind with no red gate, fix inside your own write scope; there are no
   re-frozen tests." Section 1: the row `fix-implementer-l<n>-p<k>` is named
   `fix-<implement agent>-l<n>-p<k>`.
2. **Who owns a finding** (`PL6-1`; blocks the story workflow). Section 4, `review-story-diff`: the
   owner rule becomes "`owner` as the task says; when the task names none, `test-writer` for category
   `test` or `coverage`, else `implementer` (for scope `spec`, the spec's owner)."
3. **A call moved to the background** (`PL6-8`, `probes-4-apply-C2-3`; blocks the story workflow). The
   sentence about running a command again applies to job commands only
   (`rewrite/spec/topics/long-commands.md` lists them). Section 3, the clerk, and section 7 item 3: in
   place of "M3's background sentence" `clerk.md` holds, word for word, "If a `vsdd story deliver` call
   is moved to the background, run the same command again: it re-attaches to its job. A moved call of
   any other command: wait for it, as the notice says." and the test checks those two sentences. A
   method's `## Finish` holds the re-run sentence only when its closing command is a job command; the
   `demo` method's does not, because `gate demo` returns directly.
4. **The harness tests** (`S5P-N6`; step 6 `ORDER.md` 8.1 row 7). Section 7 item 4: tests 3 and 6 of
   `universal.test.mjs` apply their assertions to every file the test already lists (the toys and
   `P/workflows/`) and never name `toy-steps`, because the story workflow deletes that toy. The toy uses
   the closings `story.js` uses: a fix step of a later review loop closes with
   `gate green --since <sha>`, a second evidence step is named `evidence-<n>`, and the survey relay
   carries `--wait 540`. Test 3's file check is narrowed as the plan says until `agents/` holds every
   roster agent.
5. **What step 4 built differently** (`PL4-23`; `rewrite/plans/step-4/ORDER.md` section 7, "Corrections
   to `rewrite/plans/step-5/agents.md`"):
   - `test-fixes`: `--refreeze` works only as the whole single-segment command of the test-writer's own
     call, never inside `cd x && ...`, and is refused without the caller marker. A changed frozen test
     that is skipped fails the re-freeze.
   - `fix-findings` step 5: when delivery's `detail` says the tool concluded the merge, there is no
     marker to resolve; run the green gate, do not return `blocked`.
   - `demo`: the evidence directory is the unit's `demo_dir` (`.factory/demo/<cycle>/<story>/`), which
     is the directory the demo gate reads.
   - M1 and the review methods: contracts arrive as `bc_paths`. A finding's `location.path` under
     `.factory/` is relative to the project root, any other path to the tree root.
6. **A plain `vsdd` call** (`ORCH-2`; `rewrite/spec/topics/caller-identity.md`). The hook runner approves
   an agent's own `vsdd` call, and attaches the caller marker, only when the whole command text is one
   plain call: `vsdd`, then arguments that are bare words of letters, digits and `_ @ % + = : , . / -`
   or single-quoted strings. Every method that tells an agent to run a `vsdd` command says to run it as
   the whole command of its own Bash call: no `cd x &&`, no pipe, no redirection, no variable.
7. **An MCP tool in a tools line** (`L2-c2-b6-fix-1`). The shipped frontmatter reader refuses a bare
   scalar that holds an underscore, so a `tools` value that names an MCP tool is written quoted
   (`tools: "Read, mcp__..."`). The lint fails a file the reader refuses, so a miss is loud.

## 4. Checks

From `next/`: `uv run --no-sync python tools/build.py`, then `tools/check.py`, the whole pytest suite,
`node --test tests/workflows/universal.test.mjs`, and `tools/build.py --check`. From the repository
root: `python3 rewrite/tools/spec_lint.py` and `python3 rewrite/tools/findings.py build`. Python runs
with `-B`.

## 5. What the story workflow needs of this unit

Rows 5 to 8 of `rewrite/plans/step-6/ORDER.md` section 5: corrections 1 to 4 above. If one does not
hold when the story workflow's builder starts, that builder stops and reports.
