---
kind: packaging
id: tests-conformance
package: G3
sources: [packaging.md#layer-2b-conformance-fixtures, BLUEPRINT.md#5.5]
---

# Test layer 2b: conformance fixtures

The convergence rule exists twice: `judge()`, `converge()` and `holdoutFindings()` in the prelude, and the Python
the tool's verdict function uses. `tests/conformance/` holds JSON cases (findings, claims, coverage, rulings, the
expected verdict). The Node tests run each case through the prelude functions, the pytest suite runs the same files
through the tool, and both must produce the expected verdict. The route rule is not duplicated: only the tool
computes it. The Python cases marked so below start from step records, because how the tool finds passes, fixes and
rulings among them ([[topic:verdicts]]) has no counterpart in the script.

Blueprint 4.14 names the prelude's `judge()` and `converge()` as the single source of the rule
([[topic:single-sources]]), and the only rendering of that code in the design is the skeleton in the pipeline
design. So the behaviours both review passes fixed are stated here as fixtures, each with its expected verdict, and
package G3 writes them before it ports the code. A port of an older reading fails them.

| Fixture | Expected |
|---|---|
| A clean first pass | `converged`, `clean`, one pass, `rounds_used` 1 |
| A pass with no blocking finding and one item not marked checked | Not clean: the gap is a finding of category `coverage` owned by the test-writer, the fixer is called, and the next pass converges |
| A finding still open at the cap, never disputed, arbiter rules `not_defect` | `blocked`, `cap`; nothing dismissed |
| The same, disputed by the builder | `converged`; one dismissal by the arbiter |
| The same, disputed, but `critical` or category `security` | `blocked`, `cap` |
| Two rounds used in earlier runs, cap 3 | One more round is allowed, then `blocked`, `cap`, `rounds_used` 3 |
| A reviewer returns blocked | `blocked`, `reviewer_blocked`; not `failed`, `no_result` |
| A must-pass scenario at 0.5 (floor 0.6) | One finding of category `holdout`, key `holdout:<HS id>`, no location: blocking |
| A mean of 0.8 (threshold 0.85) | One finding `holdout:<unit>:mean`: blocking |
| A severe finding with no location and a category other than `holdout` | Advisory |
| A reviewer pass stored with a builder's agent type, the reviewer's method echoed in its identity | The tool does not count it (Python only) |
| A severe finding marked `grounded: false` | Advisory, in both copies |
| Two loops in one run: the first converged, the second (after a repair) not | The unit's verdict is the second loop's; the first loop's records are intact (Python only) |
| A loop whose records go on after a pass with nothing open | The verdict is the one the last recorded pass gives (Python only) |
| Claims after a fix | Each has `key`, `path`, `line_start`, `line_end`, `resolution`, `commit` and nothing else; a `holdout` claim has no path |

## Scenario tests

Scenario tests, written by whoever builds each workflow, cover what the universal tests cannot: resuming (the recon
stub reports steps already done, and the test asserts no agent ran for them), each gate, each human stop, and the
spike round trip. Each of these is a path that could not run as first designed: one
scenario for each fast route of `feature` (`trivial`, `bugfix`, `hotfix`), in which the story is written after the
survey and admitted by `vsdd gate plan --story`; a fast route with two stories in which the second fails its plan
gate once, where the step is retried and both stories are delivered; the amendment run inside an open cycle; a
weak-test finding fixed by the test-writer and recorded with `vsdd gate red --refreeze`, where the strengthened test
fails until the implementer changes the code; a frozen test the implementer reports as wrong, corrected by the
test-writer before the implementer runs again; a fix round in `verify` that does not fit beside the reserve and
returns `partial`; a named spike opened, advanced and closed by the runner; and a maintenance run that delivers
fixes and opens a maintenance cycle.

Delivered by package G3 ([[package:G3]]) with the verdict function; the fixtures run in Node through the harness
([[packaging:tests-workflows]]) and in Python through the tool.
