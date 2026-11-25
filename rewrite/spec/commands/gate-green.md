---
kind: command
id: gate-green
package: G2
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.1]
---

# `vsdd gate green`

Measures build, full suite, lint, format and unchanged frozen tests on a story's head commit and writes the
diff patch.

## Syntax

`vsdd gate green --story <id> --run <id>`; `--since <commit>`. A job command ([[topic:long-commands]]).

## Pass condition

"The green gate passes when the configured build, full suite, lint and format commands pass on the head
commit, no stub marker remains, the frozen tests are unchanged since the latest red or re-freeze record,
and for a UI story the configured UI check passes. On a project with a baseline, 'pass' means no new
failure: a test, lint finding or format finding that the baseline lists does not fail the gate, and
anything else does" ([[record:baseline]]). A `docs` story's green gate "runs the documentation check
(`commands.docs_check`)" as well as the others — see [[table:story-kinds]]; a story with `ui` also needs
`commands.ui_check`, and a story whose kind or frontmatter requires a check that is not configured fails
the gate, naming it. The static checks — the red record, the frozen files, the stub marker and the
required checks — run first: if one fails, no command runs and the failing green record is written.

Two parts of the condition, made exact:

- **A red record must exist.** For a story whose kind has a red gate ([[table:story-kinds]]) the gate fails
  with `no red record` unless a passing red or re-freeze record exists on a commit between the base and the
  head ([[topic:evidence-lookup]]). "Unchanged" then means that every frozen file of that record has the
  recorded SHA-256 at the head.
- **The stub marker** is the literal string `VSDD-STUB`. The test-writer puts it in a comment on every stub
  it writes; the gate fails while any file of class `source` at the head contains it.

The worktree must be clean, as for every story gate ([[topic:gates]]).

## Who may call it

"The story's builder, `clerk`."

## Writes

"Evidence `green.<sha12>.json`; the diff patch `diff.<sha12>.patch` from the story's base — `diff.<since12>-<sha12>.patch`
when `--since` narrows the range, so a narrowed patch never replaces the whole patch the next run's full
review is given (PL4-6); the test output as `green.<sha12>.log`" — "Its result carries `diff_patch`, the
path of the patch it wrote for the head, `test_output`, and with `--since` the range's `changed_files`
(the one definition is `vsdd.state.verdicts.changed_files`); the record's `diff_patch` field and its
`patch_id` are always the base to the head" ([[record:evidence]]). The whole base-to-head patch is
written again on every call, a cached result included.

## Output and failure cases

The gate result with `diff_patch` ([[schema:gate-result]]). The flaky rule — an inherited test that fails
and then passes when the gate runs the suite once more does not fail the gate — is stated once, in
[[record:baseline]]; the gate result lists such a test in `flaky`, and the list reaches the unit's
closure record with the evidence.

Evidence lookup: "For `green` and `demo` it returns the record for the head commit, or one whose patch id
equals the head's after a clean rebase" ([[topic:evidence-lookup]]).

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path. In a project that uses feature flags the gate also applies the checks of
[[table:flag-story-checks]] whose `form` names it.
