"""The rule case tables (plan rules.md section 7, D9): one table per rule,
each case in process and — where ``launcher = true`` — through the real
launcher under sh, bash and dash, so a gate rule with a blocking case is
proven through the code the platform calls. The projects are the named
fixtures of ``rule_projects.CASE_PROJECTS``; the values a fixture returns
(``{root}``, ``{worktree}``, ``{story}``, ``{run}``, ``{sha}``) are
substituted into the payload and the ``set`` strings.
"""

from __future__ import annotations

import json
import tomllib
from pathlib import Path

import pytest

from tests.hooks import runner_helpers as helpers
from tests.hooks import rule_projects
from tests.hooks.test_runner import base_env
from vsdd.hooks import registry

TABLES = helpers.NEXT / "tests" / "hooks" / "cases"

CASE_FILES = sorted(TABLES.glob("*.toml"))


def cases() -> list[dict]:
    loaded = []
    for path in CASE_FILES:
        table = tomllib.loads(path.read_text(encoding="utf-8"))
        for case in table.get("case", []):
            case = dict(case)
            case["file"] = path.name
            loaded.append(case)
    return loaded


def substitute(value, values: dict):
    if isinstance(value, str):
        for key, replacement in values.items():
            value = value.replace("{" + key + "}", replacement)
        return value
    if isinstance(value, dict):
        return {key: substitute(item, values) for key, item in value.items()}
    if isinstance(value, list):
        return [substitute(item, values) for item in value]
    return value


def apply_set(payload: dict, changes: dict) -> dict:
    """Dotted ``set`` keys applied to the payload, as the probe runner does."""
    for dotted, value in (changes or {}).items():
        target = payload
        parts = dotted.split(".")
        for part in parts[:-1]:
            target = target[part]
        target[parts[-1]] = value
    return payload


def case_setup(tmp_path: Path, case: dict):
    """The case's project: a named fixture, or the runner's bare project."""
    fixture = case.get("project")
    if fixture is None:
        root = helpers.make_project(tmp_path)
        return root, str(root), {"root": str(root)}
    return rule_projects.case_project(fixture, tmp_path)


def case_payload(tmp_path: Path, case: dict) -> tuple[str, Path]:
    root, cwd, values = case_setup(tmp_path, case)
    payload = helpers.payload(case["payload"])
    payload["cwd"] = cwd
    body = substitute(payload, values)
    changes = substitute(case.get("set") or {}, values)
    return json.dumps(apply_set(body, changes)), root


@pytest.mark.parametrize("case", cases(), ids=[case["name"] for case in cases()])
def test_case_table_both_ways(case, tmp_path):
    """Every case passes in process; a blocking case also exits 2 through the
    real launcher under each interpreter, and an allowing one exits 0."""
    payload_text, project = case_payload(tmp_path, case)
    expect = case["expect"]

    code, _out, err = helpers.run_hook(payload_text, "gate", env=base_env(project))
    if expect["outcome"] == "block":
        assert code == 10, err
        assert f"{expect['rule']}/{expect['code']}" in err, err
    else:
        assert code == 0, err

    if case.get("launcher"):
        from tests.hooks._helpers import INTERPRETERS

        launch_sh = helpers.PLUGIN / "py" / "launch.sh"
        for interpreter in INTERPRETERS.values():
            result = helpers.launch(
                interpreter, launch_sh, ["hook", "gate"], project,
                stdin_text=payload_text,
                env={"VSDD_EVENTS_DIR": str(project / ".factory" / "local" / "events")},
            )
            want = 2 if expect["outcome"] == "block" else 0
            assert result.returncode == want, (interpreter, case["name"], result.stderr)


def test_every_rule_row_has_both_cases():
    """Each gate row of ``registry.RULES`` has a blocking and an allowing
    launcher case in the tables, and each module under ``vsdd/rules/`` has a
    row (decision D9: a gate rule with no blocking case through the real
    launcher is not proven)."""
    table = cases()
    problems: list[str] = []
    for row in registry.RULES:
        if row.kind != "gate":
            continue
        ruled = [case for case in table if case["expect"].get("rule") == row.id]
        if not any(case["expect"]["outcome"] == "block" and case.get("launcher") for case in ruled):
            problems.append(f"rule {row.id}: no blocking launcher case")
        if not any(case["expect"]["outcome"] == "allow" and case.get("launcher") for case in ruled):
            problems.append(f"rule {row.id}: no allowing launcher case")
    assert problems == [], problems

    rows = {row.id for row in registry.RULES}
    for path in sorted((helpers.PLUGIN / "py" / "vsdd" / "rules").glob("*.py")):
        if path.stem == "__init__":
            continue
        assert path.stem.replace("_", "-") in rows, f"{path.name}: no row"
