# Step 5 plan: the ten agents `story.js` names, and their fifteen methods

Paths are under `next/plugins/vsdd-factory/` unless they start with `next/` or `rewrite/`.

## 1. Scope

The steps `story.js` runs. `where` = story id, worktree, story file path, contract ids. `brief()` gives every
non-clerk step its `Method:` line first, then the project directory, the spike rule, the step identity to
return and, last, the closing command.

| Step | Given besides `where` | Closing | `data` schema |
|---|---|---|---|
| `survey`, `deliver-<n>`, `close` (the clerk) | one `vsdd` command, its schema's empty object | - | whole result: `survey`, `delivery-data`, `checkpoint-result` |
| `tests` | - | `gate red` | none |
| `implement`, `implement-2` | frozen-test rule; on `-2` the test-writer's answer | `gate green` | `implement-data` |
| `test-defects` | the implementer's `test_defects` | `gate red --refreeze` | none |
| `review-<lens>-l<n>-p<k>` | tree root, patch and commit range, test output, review rules, items to account for, claims, items not yet covered | - | `review-data` |
| `fix-test-writer-l<n>-p<k>` | blocking findings, up to 10 advisory, the green gate command | `gate red --refreeze` | `fix-data` |
| `fix-implementer-l<n>-p<k>` | the same; whether re-frozen tests now fail | `gate green` | `fix-data` |
| `arbitrate-l<n>` | open findings, the builder's dispute notes | - | `ruling-data` |
| `evidence` | - | `gate demo` | none |
| `repair-<n>` | delivery state and detail | `gate green --since <sha>` | none |

The method of each step is in the heads of section 4, its agent in section 3. Built now: the ten agent files,
the fifteen methods (D1) and what section 8 lists with them. Every other method and agent waits for its
package (D20).

## 2. The frame

Agent file `agents/<name>.md`. Seven frontmatter keys (D2), in order: `name` (the file stem); `description`
(one sentence, at most 200 characters); `model`, `effort` (the roster tier's); `tools` (one comma-separated
line); `color` (`blue` planner, `green` builder, `red` reviewer, `yellow` clerk); `omitClaudeMd: true`. Body,
in order:

1. `You are the <role> of a vsdd-factory project.` and two or three sentences of mission.
2. `## Rules`: at most eight, specific to the role; each is `(enforced)` by a hook or shows in the result. A
rule that is neither goes in a method's `## Do not`.
3. Reviewers only: `## Severity` with two empty marker pairs, `stamped:severity` then `stamped:keyrule`,
written like the pair of item 7.
4. `## Inputs`, ending: a required input that is missing means `blocked`, not a substitute.
5. `## Method`, the same words in every file but the clerk's: "Your task's first line names your method and
its file: read that file first and follow it. Your methods: `a`, `b`. A method not in this list, or a missing
file, means blocked. Dispatched without that line, read `${CLAUDE_PLUGIN_ROOT}/methods/<name>.md`." Builders
add: "If your task ends with a command to finish with, run it exactly as written and put its final JSON in
`gates` unchanged." Lint and tests read the backticked names (D3). The `${CLAUDE_PLUGIN_ROOT}` sentence rests
on probe V1; only a direct dispatch needs it (D19).
6. `## Result`: the role's fields, then the template's two closing sentences, unchanged.
7. Last, the empty pair `<!-- >>> stamped:shared -->`, `<!-- <<< stamped:shared -->`: the build's strings, not
the longer one in the spec example (D4).

The pairs stay empty: `next/tools/build.py` stamps them.

Every agent file: body at most 60 lines, file at most 70, stamped regions not counted (D7). No tool or key the
lint refuses, no model name, old-system word or history phrase.

`tools` writes out the agent's sets in the "Who gets what" table of topics/tool-sets.md, in that order (VSDD
is `Bash`). DOCS, LOOKUP and BROWSER tools in both spellings, `mcp__plugin_vsdd-factory_<server>__<tool>` and
`mcp__<server>__<tool>`; STORYBOOK only as `mcp__storybook__<tool>`; never a bare MCP name (D8).

Method file `methods/<name>.md`: no frontmatter; at most 150 lines; headings `# <name>`, `## When`,
`## Inputs`, `## Steps`, `## Do not`, `## Finish` (the checks before returning and the closing command),
`## Result`. Common text, in the same words wherever it applies:

- M1 paths. Use paths as given. An id becomes a path with `vsdd -C <project dir> path <id>`, which prints the
path alone. With no shell, Glob `**/<id>.md` under `<project dir>/.factory/specs`: the one state root an agent
may search, for contracts only; nothing found is `blocked`. No other state path is built.
- M2 builders. Project commands run as `cd <worktree> && ...` and come, with the path globs, from
`config.commands` and `config.paths` of `vsdd -C <project dir> config --json`. Git: `add`, `commit`, `rm`,
`mv`, `restore` and read commands only; no attribution line in a commit message (enforced). A git write
refused with `Operation not permitted`: `blocked`, `environment`, naming the Bash sandbox and git's line.
- M3 finish. Commit everything: a story gate measures a clean worktree at its head. Run the closing command as
given; again while `pending`. If the call is moved to the background, run the same command again: it
re-attaches to the job already running and is not a second copy; do not wait for or read the background
task. On `passed: false` act on `failures`, commit and run again, twice at most; return the last output.
Any other gate the task asks for is run once and returned as it is.
- M4 result. `step` unchanged. `head` = the closing gate's `sha` (D11); if no gate ran, the worktree's commit,
or empty with no shell. `gates` = the last output of each gate command, unchanged, one entry a gate (D12:
`gated()` reads the first of a name). `data` = exactly the fields of the schema given; other facts go in
`summary`, at most 600 characters. A stop is `needs_spike` with `spike_request`, or `blocked` with a reason
and a kind: `spec_gap` (the spec is wrong or silent), `environment` (a person must supply an input or a tool),
`conflict`, `reviewer_blocked` (any stop of a reviewer), else `other`.
- M5 stubs. Until red exists a source edit is a stub only: a new symbol's signature, a body raising the
language's not-implemented error, a `VSDD-STUB` comment, in the file where the symbol belongs. Existing
behaviour does not change.

## 3. Agents

An agent's read walls, write scope, dispatch and command groups are the row `vsdd-factory:<name>` of
`py/vsdd/policy/walls.toml`, never copied. Opening, Inputs and Result fields come from
`rewrite/spec/agents/<name>.md`, narrowed by section 1, D13 and D14.

`description` = the first sentence of the spec file's opening, less its tier word, then `Use for <when>.` In
brackets, the when and the methods the file lists now; then the rules it carries beyond its spec file:

- clerk (a step that is one command; no method): run exactly the one command, Bash timeout 600000, nothing
else (enforced); again while it prints state `pending`; M3's background sentence; never
interpret, repair or summarise; no JSON from a finished call means the empty object given, with `error` set
to the output. All four headings stay, as the lint asks (D10);
`## Method` says it has none.
- test-writer (a story's tests stage and findings about tests; `failing-tests`, `regression-test`,
`equivalence-tests`, `test-fixes`): tests come from criteria and contracts, never from the implementation;
stubs only until red exists (enforced); only it changes a frozen test, then re-freezes (enforced); the summary
names each frozen test changed, and why.
- implementer (a story's implement stage, fix passes and delivery repairs; `tdd`, `fix-findings`,
`port-translation`, `service-clone`): never edits a test (enforced), a wrong frozen test is `blocked` with
`test_defects`; every finding `fixed` with a commit or `disputed` with evidence; a new dependency is named in
the summary; no merge, rebase, push or branch (enforced).
- verifier (a harness story; `proofs`): measures nothing, the gates do; the summary names each property not
built.
- demo-recorder (a story's evidence stage; `demo`): a criterion it cannot show is reported, not faked; writes
only demo evidence and temporary files (enforced).
- technical-writer (a docs story; `docs`): the summary says how each statement was checked: in the code, or by
a command it ran.
- devops (a chore story; `repo-setup`): no push, pull request, merge, tag or worktree (enforced); it cannot
edit `factory.toml` (enforced).
- adversary (`review-story-diff`): the opening, description and Rules of the complete example in
topics/agent-file.md; all else is the frame of section 2, not the example. The key bullet reads "Give each
finding the key the rule below defines, and name its `owner`"; the read bullet keeps its words: earlier
reviews and holdout scenarios (enforced), no list of classes. Inputs add: contracts as paths, or as ids found
as the method says.
- security-reviewer (any security review; `security-review`): the adversary's rules in five bullets, and its
Inputs; each `title` starts with its CWE id; in the pipeline it writes nothing, except
`analysis/<scope>/security.md` once during an ingest, and on demand one report at the path given; unable or
unwilling to review means `blocked`, never an empty finding list.
- architect (architecture work and arbitration; `arbitrate`): the opening and Rules describe the whole role
but name no `vsdd` subcommand (its closing commands arrive in the task). Writes architecture files only
(enforced); an experiment is a spike, not a shell command (enforced); when arbitrating it writes nothing.

## 4. Methods

Each applies M1 and M4; builder methods also M2 and M3. A head names the steps that use the method, and the
kind of story; closing gate and data schema are the step's row in section 1; with no data schema `data` is
`{}`; `service-clone`, `proofs`, `docs` and `repo-setup` return `test_defects: []`. "Not" is the `## Do not`
list. A port's reference and a facade's recording are at the path the story file gives; none given: `blocked`,
`environment`; the methods of a fix, port or facade story stay short.

- `failing-tests` (`tests`, feature). (1) Read the story, each `AC-<N>` and its contract. (2) A test or more
per criterion, in files of `paths.tests`, named with `ac<N>` then a non-digit; one that passes by design
carries `green_by_design` or `wiring`. (3) The stubs the tests need to build (M5). (4) Run: each new test
fails on an assertion or the stub's error, none on a build error. Not: implement behaviour.
- `regression-test` (`tests`, fix). The test that reproduces the defect the story answers and fails for that
reason; a stub (M5) for a new symbol only. Not: fix the defect.
- `equivalence-tests` (`tests`, port). Per criterion, the reference's behaviour kept as fixtures and a test
that the port equals it; stubs (M5) for the port's entry points. Not: change the reference.
- `test-fixes` (`fix-test-writer-*`, `test-defects`). (1) Judge each finding or reported defect against the
contract: strengthen a test that cannot fail, add a missing one, correct a wrong one, or leave it and say why.
(2) Advisory findings where cheap. (3) Commit; the re-freeze runs even if no test changed. (4) A green gate
command in the task: run it once and return its output as it is (`gates` = the red entry, then the green); a
failing green sends the implementer next, so never act on it. Fix step: one `resolutions` entry per key,
`fixed` with the commit or `disputed` with evidence in `note`. `test-defects`: the summary says per test what
changed or why not. A wrong contract: `blocked`, `spec_gap`. Not: touch source (enforced), weaken or delete a
test so code passes, change a frozen test nobody named.
- `tdd` (`implement`, feature). (1) Read story, contracts, conventions; run the suite, see the frozen tests
fail. (2) One failing test at a time: least code, suite, commit; drop `VSDD-STUB` as a stub becomes real. (3)
Refactor on green; lint and format pass. A wrong frozen test: stop, `blocked` (kind `other`), list it in
`test_defects` (`path`, `test`, `reason`); otherwise `[]`. A contract no code can satisfy: `blocked`,
`spec_gap`. Not: edit, skip or delete a test (enforced), re-freeze (enforced), leave a stub marker.
- `fix-findings` (`implement`, fix; `fix-implementer-*`; `repair-<n>`). (1) Per blocking finding: reproduce,
root cause, smallest change, suite, commit; or dispute it. (2) Make failing re-frozen tests pass. (3) Advisory
findings where cheap. (4) Fix story, no findings given: the frozen regression tests are the target, as in
`tdd`. (5) Repair: `ci_failed`, reproduce from the detail and fix; `conflict`, resolve every marker in the
worktree (D18), `git add`, `git commit`, never abort, and `blocked` (kind `conflict`) only when there is none.
Not: edit a test (a wrong one is `disputed`, or `test_defects` in the implement stage), omit a key: a missing
entry reads as fixed.
- `port-translation` (`implement`, port). As `tdd` against the frozen equivalence tests, translating the
reference into this project's conventions. Reference and contract differ: stop, `blocked`, `spec_gap`.
- `service-clone` (`implement`, facade). The fake of the service the story names, from its recording:
deterministic, no call to the real service, no secret. Not: write tests.
- `proofs` (`implement`, harness). Per property id, a harness under `paths.harness` that builds with the
configured commands. Not: weaken a property, edit source or a frozen test.
- `docs` (`implement`, docs). Files of `paths.docs` for the code the story covers, each statement checked in
the code or by running it. Not: describe plans or behaviour the code does not have, change source or tests
(enforced).
- `repo-setup` (`implement`, chore). What the story lists, so the configured build, test, lint and format
commands run and pass: layout, manifests, first source and test file, CI, repository configuration. A command
or glob missing from `factory.toml`: `blocked`, `environment`, with the line to add. Not: change existing
source or tests (enforced).
- `demo` (`evidence`). No evidence directory in the task: `blocked`, kind `other` (D14). (1)
`vsdd -C <project dir> env start --subject <story> --json` gives the `url`. (2) Per user-visible criterion:
Write the script `AC-<k>.sh`, `.tape` or `.spec.ts` into the directory; run it as `cd <worktree> && ...`,
output captured in the temporary directory (a Bash write into `.factory/` is blocked); Write that output,
unchanged, as `AC-<k>.txt`; a screen that is the evidence becomes `AC-<k>.png` through the browser tool's
`filename` argument (unproven, N12). (3) `report.md`: criterion, files, result. Caps: the `[demo-evidence]`
row of `artifacts.toml`. Finish: M3 without its first sentence: commit nothing, evidence is state the tool
commits. Not: write elsewhere (enforced), edit a transcript, keep video.
- `review-story-diff` (`review-adversary-*`). (1) Read the patch, the changed files under the tree root, the
story, each contract (M1), the review rules, and the test output if a path is given. (2) Per item to account
for, those not yet covered first: does the diff meet it, and would a test fail if it did not; one `coverage`
entry, the item copied; an item not yet covered ends `checked`, or `not_checkable` with the reason. (3) Look
for unmet clauses, untested edge cases, behaviour the spec does not cover. (4) Per claim, read the code:
`resolved`, `not_resolved` or `withdrawn`. A finding: `scope` `unit`, or the level that must handle it;
`owner` `test-writer` for category `test` or `coverage`, else the builder the task names, or `implementer`
(for scope `spec`, the spec's owner); `location.path` relative to the tree root, with lines and a `quote`
copied from the file (a hook checks it); `spec_ref` or `repro` when critical or high. `reviewed_sha` = the
second commit of the range in the task (D15). Not: decide pass or fail, fix, write a file, report what it did
not read.
- `security-review` (`review-security-reviewer-*`). As `review-story-diff` through the security lens: input
handling, injection, authentication, authorisation, secrets, path and request forgery, cryptography, error
leakage, new dependencies. Category `security`, CWE id first in `title`. Each scanner entry given becomes a
finding or is dismissed with a reason. `coverage`: `checked` only for an item examined for security defects,
else `skipped` with a note. Not: put project code in a web query, return an empty list in place of a refusal.
- `arbitrate` (`arbitrate-l<n>`). Per open finding read the cited lines in the worktree, the clause or
criterion, and the builder's note; rule `defect`, `not_defect` (the reason cites clause and lines) or
`spec_gap` (the spec is silent or contradicts itself). One ruling per key. `not_defect` takes effect only on a
finding the builder disputed, never a critical or security one. Not: write anything, add findings, split the
difference.

## 5. Design decisions

D2 to D4, D10, D12, D15, D17 and D19 stand where they apply, under their numbers.

- D1. "The methods the story script gives" = its six literal names plus the default tests and implement method
of each shipped `[[story-kind]]` row.
- D5. Single sources (SL1-53): new `next/workflow-lib/finding-key.md` for the key rule, with no fallback
constant, and `agent-shared-clerk.md` for the clerk's block, bytes unchanged; the spike-runner's block is the
shared block less the first bullet of "When you cannot proceed".
- D6. Build defect: `_stamp_agents` compares the roster's tier name with the reviewer tier's model, so no
rostered reviewer is stamped. Fix: a reviewer is `agents[name] == "reviewer"`.
- D7. Lint defect: the 70-line check counts stamped text, so a 60-line body can never pass. Fix: count the
file without its stamped regions.
- D8. Lint defect: `KNOWN_TOOLS` holds bare MCP names: both spellings of section 2 are refused, and a bare
name, which never resolves and is dropped silently, passes. Fix: an MCP entry is known only in a spelling
section 2 allows its server, with the tool in that server's set.
- D9. Checks B6 has not built go in a new `next/tests/lint/test_agent_files.py`; B6's file changes only for D7
and D8.
- D11. `head` is the closing gate's `sha` even with nothing committed: `gated()` needs `sha === head`,
whatever the schema's text says (N8).
- D13. `data` holds declared fields only; other fields the agent rows name (`tests_added`, `spec_gaps`,
`harnesses`, others) go in `summary` (N2). A method used by several steps fills the schema it is given.
- D14. M1 is the one rule for a state path. The demo directory has no id, so it must be given. The test
output (empty on a first run), the review rules and the conventions path are optional.
- D16. The `blocked` of frame item 5 is kind `other` unless M4 names another: a story's own key can name a
method not built yet.
- D18. A conflict repair rests on G4a carrying out VSA-18: the tool brings the integration head into the story
branch first, leaving the markers in the story worktree.
- D20. No spike-runner in step 5: the slice cannot serve a spike, since `vsdd spike` (S6) and spike-bounds
(H8) are outside steps 3 and 4. E2's first run has no spike scenario: a spike request ends the story `failed`,
or `blocked` with `spike_limit`, and counts as a failed run.

## 6. Findings

| Finding | Outcome |
|---|---|
| P3SEAM-17 (A1) | Fixed by D2: seven keys, as L-AGENT and the spec example have them. The prose of agent-file.md, which says six: the spec steward. |
| SL1-53 (A1) | Fixed by D5, with build tests. |
| AU-S2-10 (A4) | test-writer.md and `failing-tests` state the stubs rule with no history phrase. Its other files: their packages. |
| P3BLD-24 (A5) | Followed: stamped texts stay in the build step; no method cites `assets/`, whose folders are empty (D17): A5's full build. |
| AU-P2-21 (A6a) | The lost sentence is a Rules bullet of security-reviewer.md. The wave-script sentence: W3. |
| AU-S2-22 (A6a) | Linked: no reviewer file lists its denied classes; the row defines the wall. |

New, for the orchestrator to record. N1 no conventions path, demo directory, reference
path or recording path reaches a step; the demo directory blocks every demo story (S3, W2; PL5-4, PL5-15). N2
agent rows name `data` fields no schema has (C1). N3 to N5 the defects of D6 (B4a), D7 and D8 (B6). N6
universal test 3 fails once `agents/` is partly filled (B5; section 7). N7 no spike in the slice (D20; A3, S6,
H8). N8 step-result's `head` text contradicts `gated()` (C1). N9 coverage is a union over lenses: a security
`checked` hides an adversary `skipped` (W2). N10 the fix step should use the kind's implement agent, named in
the review task: the implementer cannot fix a docs, chore or harness story (W2; PL5-13). N11 contracts should
reach agents as paths (S3, W2; PL5-2). N12: M1's Glob is proven from the project root (platform item 44),
unprobed from a story worktree, as is a browser screenshot saved into `demo/` (B8).

## 7. Tests

1. Build. `tools/build.py --check` clean. New cases in `next/tests/lint/test_build.py`: a rostered reviewer
gets both regions through `_stamp_agents` (D6), a builder neither; the clerk's block equals its source; the
spike-runner's is the full block less one bullet. `tools/check.py` expects the two new sources.
2. B6's lint. The live-tree test passes over the ten files. Fixture cases: a stamped file with a 60-line body
passes (D7); both MCP spellings pass; an unknown server and a bare tool name fail (D8).
3. `test_agent_files.py` (D9): each method an agent lists has a file, within 150 lines, with the seven
headings in order; each method file is listed by at least one agent; each agent's methods are section 3's and
its `tools` line its sets (a table in the test, until the shipped tables carry them); each `[[story-kind]]`
row's agents list its methods (its grants are B6's L-KIND); at most eight Rules bullets; every `vsdd`
subcommand the 25 files name is in `vsdd.cli.dispatch.commands()`; no L-DEAD word; none quotes a hook's block
(`hook error`, `vsdd-factory blocked`; item 40); `clerk.md` and every `## Finish` naming a gate hold M3's
background sentence (item 43).
4. Harness, `next/tests/workflows/universal.test.mjs`. A new `toys/toy-steps.js` runs each clerk step and each
method of section 4 once, through `relay()`, `step()` and `gated()`, with the agent, step name, closing and
data option `story.js` uses. For each of its calls, and each call of a file in `workflows/`, tests 3 and 6
assert: `agents/<type>.md` exists; a non-clerk prompt starts `Method: <m>.`, `methods/<m>.md` exists and the
agent lists it; each name in `schema.properties.data.required` is in backticks in the method's `## Result`; a
closing line's gate name is in its `## Finish`. The other toys keep the roster check; test 3 narrows its file
check to these calls only until `agents/` holds every roster agent, then applies it to every toy again. W2
deletes `toy-steps.js` once `story.js` is under the same assertions.

## 8. Before the builder starts, and what it must not touch

Must exist: `slice-layer-2a` finished; steps 3 and 4 built, so that `vsdd path`, `config`, `env start` and the
three story gates are registered. Step 4's `ORDER.md`, section 7, lists what it changes for the
methods. If D6, D7 or D8 is already fixed by then, the builder
keeps that fix and adds only the test. It never bends an agent file to a wrong lint and never weakens a check.

May write: the ten agent files; the fifteen method files; `next/workflow-lib/finding-key.md` and
`agent-shared-clerk.md`; the D5 and D6 changes in `next/tools/build.py` and `tools/check.py`; the D7 and D8
changes in `next/tests/lint/test_roster_lint.py`; the tests of section 7. Nothing else under `next/`: not
`prelude.js`, the text of `agent-shared.md` or `severity.md`, `roster.toml`, the policy tables or the other
toys. Nothing under `rewrite/` (new findings and outcomes go in its result) or in the old system. No commit,
branch or push.
