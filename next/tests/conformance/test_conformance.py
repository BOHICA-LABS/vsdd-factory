"""The conformance cases through the tool's Python copy of the rule
(packaging/tests-conformance.md; plan step-4 verdicts.md 3.5, decision D11).

``judge`` and ``loop`` cases run through :mod:`vsdd.state.convergence`, the
``records`` cases through ``vsdd.state.verdicts.rebuild``; the Node runner
runs the judge and loop cases through the prelude, started here once, so the
two copies of the rule cannot drift.
"""

from __future__ import annotations

import json
import pathlib
import subprocess
import sys

import pytest

from vsdd.schema import is_valid, load_registry
from vsdd.state import convergence, verdicts

HERE = pathlib.Path(__file__).resolve().parent
CASES = HERE / "cases"
ROOT = HERE.resolve().parent.parent

#: The rows of the fixture table in packaging/tests-conformance.md, each held
#: by the case or cases named for it (plan step-4 verdicts.md 7,
#: test_spec_rows_have_cases). A new row needs a row here.
SPEC_ROWS = {
    "A clean first pass": ["l01-clean-first-pass"],
    "A pass with no blocking finding and one item not marked checked": [
        "l02-coverage-gap-then-clean",
        "r04-lens-coverage",
    ],
    "A finding still open at the cap, never disputed, arbiter rules not_defect": [
        "l03-cap-undisputed-kept",
    ],
    "The same, disputed by the builder": ["l04-cap-disputed-dismissed", "r10-arbitration-record"],
    "The same, disputed, but critical or category security": ["l05-cap-disputed-critical", "l06-cap-disputed-security"],
    "Two rounds used in earlier runs, cap 3": ["l07-used-across-runs", "r09-used-from-earlier-runs"],
    "A reviewer returns blocked": ["l08-reviewer-blocked", "r13-reviewer-blocked-record"],
    "A severe finding with no location and a category other than holdout": [
        "j03-no-location-advisory",
    ],
    "A reviewer pass stored with a builder's agent type, the reviewer's method echoed in its identity": [
        "r03-builder-echoes-method",
    ],
    "A severe finding marked grounded: false": ["j05-grounded-false-advisory"],
    "Two loops in one run: the first converged, the second (after a repair) not": [
        "r05-second-loop-open",
    ],
    "A loop whose records go on after a pass with nothing open": [
        "r20-walk-on-after-clean",
    ],
    "Claims after a fix": ["l09-claims-shape"],
}

#: The holdout-score rows of the spec table: the slice defers both to G6a,
#: where holdout findings are scored in Python too (plan step-4 verdicts.md 1;
#: S4R-verdicts-9). They name no case until then.
DEFERRED_TO_G6A = (
    "A must-pass scenario at 0.5 (floor 0.6)",
    "A mean of 0.8 (threshold 0.85)",
)


def _load() -> list[dict]:
    return [json.loads(path.read_text(encoding="utf-8")) for path in sorted(CASES.glob("*.json"))]


def _registry() -> dict:
    return load_registry(ROOT / "plugins" / "vsdd-factory" / "schemas")


def _verdict_errors(verdict) -> list[str]:
    from vsdd.schema import validate

    return [
        f"{problem.path}: {problem.message}"
        for problem in validate(verdict, _registry()["verdict.schema.json"], _registry())
    ]


@pytest.mark.parametrize(
    "case", [case for case in _load() if case["kind"] in ("judge", "loop")], ids=lambda c: c["name"]
)
def test_case(case):
    """One case through the Python copy; the same file runs in Node."""
    if case["kind"] == "judge":
        input_ = case["input"]
        result = convergence.judge(input_["findings"], input_["perimeter"], input_["waived"])
        assert result == case["expect"]["result"]
        return
    trace: list = []
    verdict = convergence.replay(case["input"], trace)
    assert verdict == case["expect"]["verdict"], case["name"]
    if "trace" in case["expect"]:
        assert trace == case["expect"]["trace"], case["name"]


def test_every_case_is_well_formed():
    """``name`` is the file stem, every expected verdict is valid against
    ``verdict.schema.json``, no case carries an unknown kind, and a records
    case never repeats an identity the store keeps once (S4R-verdicts-C1-2):
    ``steps_list`` can only return one record per
    ``(run, workflow, subject, name, attempt)``."""
    kinds = {"judge", "loop", "records"}
    seen = set()
    for path in sorted(CASES.glob("*.json")):
        case = json.loads(path.read_text(encoding="utf-8"))
        assert case["name"] == path.stem, f"{path.name} does not name itself"
        assert case.get("about"), f"{path.name} carries no about line"
        assert case["kind"] in kinds, f"{path.name}: unknown kind {case['kind']!r}"
        if case["kind"] in ("loop", "records"):
            problems = _verdict_errors(case["expect"]["verdict"])
            assert not problems, f"{path.name}: {problems}"
        if case["kind"] == "records":
            identities = [
                (
                    record["step"].get("run", ""),
                    record["step"].get("workflow", ""),
                    record["step"].get("subject", ""),
                    record["step"].get("name", ""),
                    record["step"].get("attempt", 0),
                )
                for record in case["input"]["records"]
            ]
            assert len(identities) == len(set(identities)), (
                f"{path.name}: a step record repeats an identity the store keeps once"
            )
        seen.add(case["name"])
    assert seen == {path.stem for path in CASES.glob("*.json")}


def test_spec_rows_have_cases():
    """Each row of the fixture table the spec keeps names a case file, and the
    rows deferred to G6a name none (they are listed in DEFERRED_TO_G6A)."""
    stems = {path.stem for path in CASES.glob("*.json")}
    for row, names in SPEC_ROWS.items():
        assert names, f"the spec row {row!r} names no case"
        assert row not in DEFERRED_TO_G6A
        for name in names:
            assert name in stems, f"the spec row {row!r} names the missing case {name!r}"


def test_node_agrees():
    """The Node runner passes the same judge and loop cases against the
    prelude; missing node is a failure, not a skip."""
    proc = subprocess.run(
        [sys.executable, "-c", "import shutil, sys; sys.exit(0 if shutil.which('node') else 1)"],
        capture_output=True,
    )
    assert proc.returncode == 0, "node is not available; the conformance run cannot be trusted"
    proc = subprocess.run(
        ["node", "--test", str(HERE / "conformance.test.mjs")],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=300,
    )
    assert proc.returncode == 0, (
        f"the Node runner disagrees:\n{proc.stdout}\n{proc.stderr}"
    )


# --------------------------------------------------------------- records --->


def _changed_from(mapping):
    """The case's ``changed`` map as the callable ``rebuild`` expects: keys
    are ``<since>..<head>``; a miss reads as None (git cannot say)."""
    return lambda since, head: mapping.get(f"{since}..{head}")


@pytest.mark.parametrize("case", [case for case in _load() if case["kind"] == "records"],
                         ids=lambda c: c["name"])
def test_records_case(case):
    input_ = case["input"]
    rebuilt = verdicts.rebuild(
        input_["records"],
        subject=input_["subject"],
        cap=input_["cap"],
        waived=input_.get("waived", []),
        acs=input_.get("acs", []),
        changed=_changed_from(input_.get("changed", {})),
    )
    got = {
        "verdict": rebuilt["verdict"],
        "loops": [
            {"run": loop["run"], "loop": loop["loop"],
             "outcome": loop["verdict"]["outcome"], "reason": loop["verdict"]["reason"]}
            for loop in rebuilt["loops"]
        ],
        "skipped": [
            {"step": skipped["step"], "why": skipped["why"]} for skipped in rebuilt["skipped"]
        ],
        "chain": rebuilt["chain"],
        "passes_used": rebuilt["passes_used"],
    }
    assert got == case["expect"], case["name"]
    problems = _verdict_errors(rebuilt["verdict"])
    assert not problems, f"{case['name']}: {problems}"
    assert is_valid(
        rebuilt["verdict"], _registry()["verdict.schema.json"], _registry()
    )
