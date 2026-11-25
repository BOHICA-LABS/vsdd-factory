"""The case table: every case runs in process, and the ones marked
``launcher`` also run through the real launcher under sh, bash and dash
(plan 7; hooks.md 2.14). Case tables live in ``tests/hooks/fixture_cases/``
(this unit's probe table). The per-rule tables of step 4,
``tests/hooks/cases/<rule>.toml``, are the rules unit's to run
(``test_rule_tables.py``); this file reads them as data only, so the
coverage check below still sees every registered gate (S4B-rules-4)."""

from __future__ import annotations

import json
import tomllib
from pathlib import Path

import pytest

from tests.hooks import runner_helpers as helpers
from tests.hooks.test_runner import base_env

_CASE_DIRS = (
    Path(helpers.PROBE_TABLE).parent,  # fixture_cases/
)

#: Where the shipped rule modules live (HD1): the directory the
#: modules-to-rows check reads.
RULES_DIR = helpers.PLUGIN / "py" / "vsdd" / "rules"

#: The per-rule tables of step 4, read as data by the coverage check.
_RULE_TABLE_DIR = helpers.NEXT / "tests" / "hooks" / "cases"

CASE_FILES = sorted(path for directory in _CASE_DIRS for path in directory.glob("*.toml"))


def cases() -> list[dict]:
    loaded = []
    for path in CASE_FILES:
        table = tomllib.loads(path.read_text(encoding="utf-8"))
        for case in table.get("case", []):
            case = dict(case)
            case["file"] = path.name
            loaded.append(case)
    return loaded


def rule_table_cases() -> list[dict]:
    """The step 4 per-rule tables, counted by rule and expectation but never
    run here: ``test_rule_tables.py`` runs them (S4B-rules-4)."""
    loaded = []
    for path in sorted(_RULE_TABLE_DIR.glob("*.toml")):
        table = tomllib.loads(path.read_text(encoding="utf-8"))
        for case in table.get("case", []):
            case = dict(case)
            case["file"] = path.name
            loaded.append(case)
    return loaded


def apply_set(payload: dict, changes: dict) -> dict:
    for dotted, value in (changes or {}).items():
        target = payload
        parts = dotted.split(".")
        for part in parts[:-1]:
            target = target[part]
        target[parts[-1]] = value
    return payload


def case_payload(case: dict, project: Path) -> str:
    if case.get("payload") == "broken":
        return "garbage{"
    payload = helpers.payload(case["payload"])
    payload["cwd"] = str(project)
    return json.dumps(apply_set(payload, case.get("set")))


@pytest.mark.parametrize("case", cases(), ids=[case["name"] for case in cases()])
def test_case_table_both_ways(case, tmp_path):
    """Every case in the probe table passes in process; a blocking case also
    exits 2 through the real launcher, and an allowing one exits 0."""
    project = helpers.make_project(tmp_path)
    expect = case["expect"]
    payload_text = case_payload(case, project)

    code, out, err = helpers.run_hook(payload_text, "gate", env=base_env(project))
    if expect["outcome"] == "block":
        assert code == 10, err
        assert f"{expect['rule']}/{expect['code']}" in err, err
    else:
        assert code == 0, err
        if expect.get("rewritten"):
            assert "VSDD_CALLER=" in out, out
        if expect.get("decision"):
            rendered = json.loads(out)["hookSpecificOutput"]
            assert rendered["permissionDecision"] == expect["decision"]

    if case.get("launcher"):
        from tests.hooks._helpers import INTERPRETERS

        launch_sh = helpers.PLUGIN / "py" / "launch.sh"
        for interpreter in INTERPRETERS.values():
            result = helpers.launch(
                interpreter, launch_sh, ["hook", "gate"], project,
                stdin_text=payload_text,
                env={"VSDD_EVENTS_DIR": str(project / "events")},
            )
            want = 2 if expect["outcome"] == "block" else 0
            assert result.returncode == want, (interpreter, result.stderr)


def test_gate_case_coverage():
    """Every gate-blocking core code has a case that exits 2 through the
    launcher, and one that passes (hooks.md 2.14; the meta-test's slice-depth
    core); every shipped gate rule has a blocking and an allowing launcher
    case, and a case naming a module names the row's module."""
    table = cases() + rule_table_cases()
    blocking = {case["expect"]["code"] for case in table if case["expect"]["outcome"] == "block"}
    launcher_codes = {case["expect"].get("code") for case in table
                      if case.get("launcher") and case["expect"].get("code")}
    assert {"caller_forged", "runner_bad_input"} <= blocking
    assert "caller_forged" in launcher_codes  # the loud failure, launcher-proven
    allowing = [case for case in table if case["expect"]["outcome"] == "allow"]
    assert any(case["expect"].get("rewritten") for case in allowing)

    # Per rule (hooks.md 2.14): a gate rule is covered by one blocking and one
    # allowing launcher case, a case that names a module names the module the
    # row loads, and the modules and rows correspond. Vacuous while RULES is
    # empty (step 4 ships the rows); the functions are tested below with rows
    # by hand, so the checks do not pass vacuously meanwhile.
    from vsdd.hooks import registry

    found = gate_case_problems(registry.RULES, table)
    assert found == [], found
    found = module_row_problems(registry.RULES, RULES_DIR)
    assert found == [], found


def loaded_module(row) -> str:
    """The module a row loads: its own, or ``RuleEntry.load``'s default from
    the id (model.py: ``vsdd.rules.<id with dashes as underscores>``)."""
    return row.module or "vsdd.rules." + row.id.replace("-", "_")


def gate_case_problems(rows, table) -> list[str]:
    """The coverage faults of one case table against the rule rows (hooks.md
    2.14): a gate row with no blocking or no allowing launcher case, and a
    case whose module is not the one its row loads."""
    problems: list[str] = []
    table = list(table)
    for row in rows:
        if row.kind != "gate":
            continue
        ruled = [case for case in table if case["expect"].get("rule") == row.id]
        blocks = [case for case in ruled
                  if case["expect"]["outcome"] == "block" and case.get("launcher")]
        allows = [case for case in ruled
                  if case["expect"]["outcome"] == "allow" and case.get("launcher")]
        if not blocks:
            problems.append(f"rule {row.id}: no blocking launcher case")
        if not allows:
            problems.append(f"rule {row.id}: no allowing launcher case")
        loads = loaded_module(row)
        for case in ruled:
            module = case.get("module")
            if module is not None and module != loads:
                problems.append(
                    f"case {case['name']}: module {module!r} does not match "
                    f"the row's {loads!r}"
                )
    return problems


def module_row_problems(rows, rules_dir) -> list[str]:
    """Modules and rows correspond (plan 7: 'modules match rows'): every
    ``vsdd/rules`` module has a row, and every row names a ``vsdd.rules``
    module whose file is there."""
    problems: list[str] = []
    rules_dir = Path(rules_dir)
    named = {loaded_module(row) for row in rows}
    for path in sorted(rules_dir.glob("*.py")):
        if path.stem == "__init__":
            continue
        module = f"vsdd.rules.{path.stem}"
        if module not in named:
            problems.append(f"module {module}: no registry row")
    for row in rows:
        module = loaded_module(row)
        if not module.startswith("vsdd.rules."):
            problems.append(f"rule {row.id}: module {module!r} is not a vsdd.rules module")
            continue
        path = rules_dir / (module.rsplit(".", 1)[-1] + ".py")
        if not path.is_file():
            problems.append(f"rule {row.id}: module {module} has no file in vsdd/rules/")
    return problems


def test_gate_case_problems_catches_gaps():
    """The coverage check bites on real rows: an uncovered gate row, a covered
    one, and a case whose module is not the one the row loads (S3R-8: RULES
    is empty at this depth, and an empty loop proves nothing)."""
    from vsdd.hooks.model import RuleEntry

    covered = RuleEntry(id="covered", on=(("PreToolUse", ("Bash",)),),
                        kind="gate", fail="closed")
    uncovered = RuleEntry(id="uncovered", on=(("PreToolUse", ("Bash",)),),
                          kind="gate", fail="closed")
    table = [
        {"name": "hits", "expect": {"rule": "covered", "outcome": "block"}, "launcher": True},
        {"name": "passes", "expect": {"rule": "covered", "outcome": "allow"}, "launcher": True},
        # A defaulted module name must read as the row's own (its row is
        # silent, and RuleEntry.load fills the default in):
        {"name": "defaults", "expect": {"rule": "covered", "outcome": "allow"},
         "launcher": True, "module": "vsdd.rules.covered"},
    ]

    assert gate_case_problems((covered,), table) == []

    problems = gate_case_problems((uncovered,), table)
    assert "rule uncovered: no blocking launcher case" in problems, problems
    assert "rule uncovered: no allowing launcher case" in problems, problems

    stray = {"name": "strays", "expect": {"rule": "covered", "outcome": "allow"},
             "launcher": True, "module": "vsdd.rules.elsewhere"}
    problems = gate_case_problems((covered,), table + [stray])
    assert problems == [
        "case strays: module 'vsdd.rules.elsewhere' does not match "
        "the row's 'vsdd.rules.covered'"
    ], problems


def test_modules_match_rows(tmp_path):
    """The modules-to-rows check: a module file with no row, and a row whose
    module file is missing, are both faults."""
    from vsdd.hooks.model import RuleEntry

    rules_dir = tmp_path / "rules"
    rules_dir.mkdir()
    (rules_dir / "__init__.py").write_text("", encoding="utf-8")
    (rules_dir / "present.py").write_text("", encoding="utf-8")

    orphaned_row = RuleEntry(id="orphaned", on=(("PreToolUse", ("Bash",)),),
                             kind="gate", fail="closed")  # vsdd.rules.orphaned: no file
    problems = module_row_problems((orphaned_row,), rules_dir)
    assert "module vsdd.rules.present: no registry row" in problems, problems
    assert "rule orphaned: module vsdd.rules.orphaned has no file in vsdd/rules/" in problems, \
        problems

    present_row = RuleEntry(id="present", on=(("PreToolUse", ("Bash",)),),
                            kind="gate", fail="closed")
    outsider = RuleEntry(id="outside", on=(("PreToolUse", ("Bash",)),),
                         kind="gate", fail="closed", module="tests.hooks.fixture_rules.blocking")
    problems = module_row_problems((present_row,), rules_dir)
    assert problems == [], problems

    problems = module_row_problems((present_row, outsider), rules_dir)
    assert "rule outside: module 'tests.hooks.fixture_rules.blocking' is not a vsdd.rules module" \
        in problems, problems
