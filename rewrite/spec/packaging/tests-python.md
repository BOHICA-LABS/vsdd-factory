---
kind: packaging
id: tests-python
sources: [packaging.md#layer-1-python-tests-pytest-through-uv, BLUEPRINT.md#5.5]
---

# Test layer 1: Python tests (pytest through `uv`)

`pyproject.toml` sets `pythonpath = ["plugins/vsdd-factory/py"]`, so tests import the shipped package directly.

Directories and what they test:

- `tests/unit/` — the shared core (project detection, configuration loader, frontmatter reader, event writer and
  reader) and every `vsdd` subcommand. Git behaviour is tested against real temporary repositories. `gh` is a test
  double on `PATH` (`tests/e2e/fakebin/gh`, shared with layer 4). Docker calls sit behind one function that tests
  replace.
- `tests/hooks/` — every rule, two ways: in-process (rule function, payload, temporary project, decision) and
  **through the real launcher** (`sh py/launch.sh hook` with the payload on stdin, asserting exit code, stderr and
  stdout). A meta-test walks the rule registry and fails if any blocking rule lacks a through-the-launcher case that
  blocks and one that allows. This is the test that would have caught the old `protect-bc`, which decided to block
  and never told Claude Code.
- `tests/hooks/` failure modes — `uv` missing from `PATH` exits 2 with the message; a rule that raises behaves as the
  hooks design says; an unknown tool (`Workflow`, `StructuredOutput`) passes; a project that is not initialised gets
  exit 0 and no output. These run on Linux and macOS, on the Python floor and on the newest stable CPython.
- `tests/hooks/` latency — twenty launches of the hook entry in an uninitialised project. Fails if the median exceeds
  400 ms; warns above 150 ms. Every tool call pays this cost.

The frontmatter reader is a small subset parser that ships in the plugin. A differential test parses every
frontmatter block in the plugin (agents, skills, templates), and every artifact frontmatter block in the fixture
overlays and the hook case files, which is the reader's real input, with both the shipped reader and PyYAML, and
fails on any difference. A second corpus holds inputs the reader must refuse: each is one PyYAML itself rejects or
reads as a value that differs from the text ([[topic:hook-runner]]). PyYAML is a development dependency only.

The payload format, the rule registry and the per-rule cases belong to the hooks design ([[topic:rule-testing]]).
This design supplies the directories, the launcher test, the meta-test and the matrix.

## Fixture tests

Fixture tests in `tests/unit/`, each a small state tree and one call.

- **Approve, survey, plan** (package S4) — a `spec` decision is settled; the next `plan` survey finds the approval
  current and the specification locked. Settling wrote the decision record and nothing else.
- **Assess, brief approved, spec** (package S3) — `discover` in assess mode ends; the brief is approved; the `spec`
  survey sees an open cycle.
- **The route table, one fixture per row** (package S3) — among them: a brownfield project with a queued request
  gives `feature` with that request; a greenfield project after its release gives done, not `verify`; a standing
  `ideas` decision plus a queued request gives `feature`; a wave gate at its cap gives "human" with that decision on
  the first try; a `spec-gap` settled `amend` with nothing else runnable gives `feature` with `amend`; a brief in
  draft gives the brief skill; missing commands give the lines to add; a fast route whose story is delivered gives
  `verify`, not `plan`; a maintenance cycle with delivered fixes gives `verify`; a standard-route feature whose delta
  specification is approved, with no run records on disk (a fresh clone), gives `plan`, because the route is read
  from `cycle.json`.
- **The preconditions, one fixture per predicate** (package S3) — the pipeline design's precondition predicates.
- **The decision grammar** (package S4) — every documented choice parses to the right values: repeated `key=`,
  `item=`, `name=`, `stage=`, text after `--`, a qualified id; an ambiguous bare id is refused with the choices.
- **Import, then one fast-route feature** (package S5) — after `vsdd import` on a fixture cut from a real project,
  the route is `feature` with the oldest queued request when an epic was in flight, and steady state when none was;
  either way `feature` starts.
- **Class precedence** (package S1) — `src/parser/tests.rs` is in `tests`; a path in no class is reported.
- **The baseline comparison** (package G2) — a test that failed at the baseline does not fail the green gate; a new
  failure does; the flaky rule for an inherited test is the one [[record:baseline]] states, and the conformance
  fixture follows it: the test runs once more, passes, and is recorded as flaky in the evidence and then in the
  unit's closure record, where `vsdd report debt` lists it.
- **Holdout thresholds** (package G3) — a must-pass scenario under its floor and a mean under the threshold each
  produce a blocking finding.
- **Re-attaching jobs** (package G1) — the same long command run twice starts one job; a second process re-attaches.

## Harvesting

The old bats suites are deleted, but the cases in them are data. For each rule or tool that ports, its builder
converts the old cases into table rows before the old file goes: for example the command strings in
`destructive-guard.bats` and `hooks.bats`, and the 81 cases in `factory-obs.bats`.
