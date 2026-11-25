---
kind: packaging
id: sample-project
package: E1
sources: [packaging.md#layer-4-end-to-end-headless, BLUEPRINT.md#5.5]
---

# The sample project

The sample project, `tally`, is a Python package of one module with standard-library `unittest` tests. Python is
chosen over the old Rust fixture because `uv` is already required, so the sample needs no extra toolchain.

The red gate reads per-test results from a JUnit report, and `unittest` writes none, so the sample ships its own
runner, `tools/junit_runner.py`: about sixty lines of standard library that run the suite and write JUnit XML. Its
test command is `uv run python tools/junit_runner.py` and its `test_report` is `junit:build/junit.xml`. A
standard-library runner is chosen over `pytest --junitxml` because the end-to-end tiers must not need a package
download: a PyPI fetch failed in the sandboxed shell during the platform spike.

The report is written after every run and never quietly loses a test. An import-broken test module — however it
fails to import: a missing module, a syntax error, a name error — becomes an `ImportError` case in the report,
not a runner crash without a report. Discovery collects `TestCase` classes only, so the runner also collects
every module-level `test_` function of a test module — under `tests/` and in the `test_*.py` files beside the
product's package — with the identity and file a class of the same module gets; a test function the runner cannot
run as written, with required parameters or a coroutine, async-generator or generator function whose answer would
never be awaited or iterated, is reported as an error case. The gate tests' project builder builds this
sample through the story-ready loader and asserts the built configuration — `delivery.mode` `local`,
`paths.tests` and `paths.source` as the sample's own `factory.toml` has them — so a regression there is a loud
failure, not a misread gate.

Its origin is a local bare repository. The sample exists in five shapes, as fixture overlays:

- `story-ready`: the package with its tests, initialised, with one admitted story (E2,
  [[packaging:e2e-story-tier]]), defined below;
- an empty repository with a brief (E3, [[packaging:proof-greenfield]]);
- the package with its tests and one known lint failure and no factory state (E4, ingest,
  [[packaging:proof-brownfield]]);
- the package with no tests and no CI (E4);
- two small repositories under a hub (E4).

Pull requests go through `tests/e2e/fakebin/gh`, a Python stand-in for the `gh` commands the pipeline uses, backed
by that bare repository and a JSON file. A command it does not implement fails with "unsupported in the test double"
and is logged, so a gap shows up as a clear failure.

## The `story-ready` fixture

A standalone story needs more state than a file copy can make, so the overlay is data and a loader builds
the project from it. The overlay holds the `.factory/` files:

- `factory.toml` with `[commands]` and `[paths]` set, because detection cannot know the sample's own runner
  (`test = "uv run python tools/junit_runner.py"`, `test_report = "junit:build/junit.xml"`), and with
  `delivery.mode`;
- `cycles/<cycle>/cycle.json` with the route `standard`;
- an approved `spec` decision, whose digest the loader computes;
- the plan's closure record with `waves = { "W-01": ["S-1.01"] }`, and no wave closure;
- one contract, the epic and the story `stories/E-1/S-1.01.md`.

`tests/e2e/fixtures/load.py <overlay> <directory> [--local]` copies the sample, runs `vsdd init`, replaces
the detected `factory.toml` with the overlay's, copies the other files onto the state branch, computes the
digest and commits. With `--local` it removes the remote before `vsdd init` runs, so the project is in local
delivery mode; the checkout stays on the integration branch, which is the case local delivery is built for
([[command:story-deliver]]). The first run of the story tier uses `--local`.

Each fixture overlay is checked with `vsdd validate` in the unit tests, so fixtures cannot rot as the state schema
changes.

Delivered by package E1 ([[package:E1]]), ahead of its probe runner and smoke probes ([[packaging:build-plan]]);
used by the story tier and both proofs ([[packaging:e2e-story-tier]]).
