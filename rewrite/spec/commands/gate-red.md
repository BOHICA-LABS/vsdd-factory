---
kind: command
id: gate-red
package: G2
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.9.1]
---

# `vsdd gate red`

Measures that a story's new tests fail on assertions and freezes their identities; `--refreeze` records
changed frozen tests.

## Syntax

`vsdd gate red --story <id> --run <id>`; `--refreeze`. A job command ([[topic:long-commands]]).

## Pass condition

"The red gate passes when the stubs build, the new tests fail on assertions and not on build errors, and at
least half of the non-exempt new tests are red. The exemptions carry over: tests that are green by design
and wiring tests. The gate reads per-test results from the JUnit report named by `commands.test_report`.
New tests are the test identities missing from the report of the base commit. The gate records the identity
and file hash of every new test; those are the frozen tests. Which file is a test file follows the class
precedence: a path that matches a tests glob is in `tests` even when a source glob also matches, so the
tests in `src/parser/tests.rs` count and the test-writer may still edit that file after red. A test defined
in a file classed as `source` does not count." (Story kinds without a red gate: [[table:story-kinds]],
[[rule:tests-first]].)

## What the gate reads

- **Test identity.** `<classname>::<name>` of each `<testcase>` in the JUnit report.
- **The base report.** The identities at the story's recorded base ([[topic:worktrees]]). The gate runs the
  test command once on that commit, in a temporary detached worktree the worktree library makes and
  removes, and keeps the report as `local/evidence/_base/<sha12>.xml`; every story cut from the same commit
  reuses it. When the test command produces no report at the base, the base set is empty.
- **A red test.** A new test whose `<testcase>` holds a `<failure>`, or an `<error>` whose `type` ends in
  one of the not-implemented types held as a constant in code (`NotImplementedError`,
  `NotImplementedException`, `UnsupportedOperationException`): a stub that raises is the expected red. Any
  other `<error>`, and an `<error>` with no `type`, is a build or harness error, and one of those among
  the new tests fails the gate, named with the identity and the type.
- **Exempt tests.** A new test whose name contains `wiring` or `green_by_design`, compared without case.
  At least half of the non-exempt new tests must be red, and at least one test must be red, so exemptions
  alone never pass the gate.
- **The file of a test.** The `file` attribute of its `<testcase>` when the report has one; otherwise the
  longest prefix of the classname (`.` and `::` read as `/`) that equals, or ends, the extension-less path
  of exactly one `tests`-class file — the file itself, extension included; two candidates name no file
  (VSA-12). A test whose file is classed `source` does not count. A test with no file found counts;
  `vsdd init` and `vsdd doctor` warn when the report names no files, because the exclusion then cannot be
  applied.
- **Frozen files.** Every file of class `tests` that the diff from the base to the head adds or changes.
  The record lists each with its SHA-256, and the new test identities ([[record:evidence]]).
- **Criteria.** A test covers criterion `AC-<N>` when its name contains `ac<N>` followed by a non-digit,
  compared without case (`test_ac2_rejects_empty` covers AC-2; `ac12` does not cover AC-1). The record
  keeps the map from criterion to test identities, which is what [[command:gate-demo]] builds its table
  from.

The worktree must be clean, as for every story gate ([[topic:gates]]).

## Re-freezing

"A review finding of category `test` ... is the test-writer's to fix, and the hook lets it change a frozen
test. The green gate would then fail on 'frozen tests unchanged'. So the test-writer's fix step ends with
`vsdd gate red --refreeze --story <id>`, which only the test-writer may run: it requires every changed
frozen test to still exist and to either pass or fail on an assertion, as the red gate does, and records the
new hashes as a re-freeze record on the head commit."

Made exact, as the code runs:

- The re-freeze needs a red record; without one it fails with `no red record`. A refused re-freeze writes
  no record, so the earlier one stands. A passing record for the head is returned as it is.
- The frozen set is computed afresh from the base to the head. A file the earlier record froze that is no
  longer in it is the failure `frozen test file removed`.
- When no file changed or was added, the record is written at the head with the earlier tests and
  criteria, and the suite does not run: an unchanged test broken by half-done code is not the
  test-writer's to mend.
- Otherwise the suite runs once. Each earlier identity must still be reported. Each earlier identity of a
  changed file, and each with no file, must pass or fail on an assertion: a skipped one fails the
  re-freeze, and so does a build or harness error among them. Each new test in a changed or unknown file
  must pass or be red as the red gate judges red; the half rule does not apply to a re-freeze. An
  unchanged test broken by half-done code does not refuse the re-freeze.
- The record carries the complete frozen set, tests and criteria at the head, with `refreeze: true`
  (finding P3SEAM-15). The gate itself does not re-check who may run it twice over: `--refreeze` is
  honoured only as the whole single-segment command of the caller, because only such a call carries the
  caller marker ([[topic:caller-identity]]), and the caller's row must hold `refreeze`.

## Who may call it

"`test-writer`, `clerk`; `--refreeze` the `test-writer` only."

## Writes

"Evidence `red.<sha12>.json` with the frozen test identities and hashes, read from the JUnit report"; with
`--refreeze`, "a re-freeze record with the new hashes of frozen tests the test-writer changed, written only
if each still exists and either passes or fails on an assertion" ([[record:evidence]]).

## Output and failure cases

The gate result ([[schema:gate-result]]), pending-shaped while the job runs
([[topic:long-commands]]). Evidence lookup for later gates: "For `red` it returns the latest passing
record, or re-freeze record, on any commit between the story's base and its head" ([[topic:evidence-lookup]]).

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path. For a story that names a feature flag the gate follows F-STATES in
[[table:flag-story-checks]]: a story that ships behind a flag needs tests in three states, and a story that
removes a flag has rules of its own there.
