"""The gate commands on the command line (plan gates.md 3.2, D1, PL4-16)."""

import json
from pathlib import Path

from tests.unit import gate_project
from tests.unit.gate_project import STORY, run_gate
from vsdd.cli.dispatch import commands

IMPL = gate_project.IMPLEMENTER


def test_three_gates_register():
    table = commands()
    for name in ("red", "green", "demo"):
        assert ("gate", name) in table
        command = table[("gate", name)]
        assert command.needs == "mounted"
        assert command.output == "gate-result"
        assert command.failure == {"gate": name}


def test_pending_then_result(tmp_path):
    root, run = gate_project.make_project(tmp_path)
    gate_project.red_commit(root)
    env = gate_project.env_with(root, IMPL)
    code, pending, _err = run_gate(
        root, "gate", "red", "--story", STORY, "--run", run, "--wait", "0", env=env
    )
    assert code == 0
    assert pending["ok"] is True and pending["state"] == "pending" and pending["retry"]
    # the job runs on; the next call with a real wait returns the result
    code2, result, _err = run_gate(
        root, "gate", "red", "--story", STORY, "--run", run, "--wait", "120", env=env
    )
    assert code2 == 0 and result["passed"] is True, result["failures"]
    assert "state" not in result


def test_usage_error_prints_gate_shape(tmp_path):
    root, run = gate_project.make_project(tmp_path)
    gate_project.red_commit(root)
    env = gate_project.env_with(root, IMPL)
    code, result, _err = run_gate(
        root, "gate", "red", "--story", "not-a-story", "--run", run, env=env
    )
    assert code == 2
    assert result["ok"] is False and result["gate"] == "red"
    assert result["error"]
    for field in ("subject", "passed", "sha", "evidence", "failures"):
        assert field in result
    code2, _result2, err2 = run_gate(
        root, "gate", "red", "--run", run, env=env  # --story missing
    )
    assert code2 == 2 and "story" in err2


def test_result_of_another_run_is_not_served(tmp_path):
    root, run = gate_project.make_project(tmp_path)
    gate_project.red_commit(root)
    env = gate_project.env_with(root, IMPL)
    # A starts the job and does not collect: the result carries run A privately
    code, pending, _err = run_gate(
        root, "gate", "red", "--story", STORY, "--run", run, "--wait", "0", env=env
    )
    assert code == 0 and pending["state"] == "pending"
    _wait_for_job(root)
    # run A ends; run B takes the story
    from vsdd.state.runs import run_end

    run_end(root, run)
    run_b = _survey_again(root)
    # no cached record: the attach path is the only path
    record = Path(root) / ".factory" / "local" / "evidence" / STORY / "red.*.json"
    for path in record.parent.glob("red.*.json"):
        path.unlink()
    env_b = gate_project.env_with(root, IMPL)
    code_b, result_b, _err = run_gate(
        root, "gate", "red", "--story", STORY, "--run", run_b, "--wait", "120", env=env_b
    )
    assert code_b == 0 and result_b["passed"] is True, result_b["failures"]
    assert "run" not in result_b, "the private run key is never printed"
    # B was not served A's result: the records were deleted, so the only way
    # B can hold a passing red record is that the job ran again for B.
    fresh = list((Path(root) / ".factory" / "local" / "evidence" / STORY).glob("red.*.json"))
    assert fresh, "the job measured again for B; A's collected result was not served"
    record = json.loads(fresh[-1].read_text())
    assert record["passed"] is True
    _ = _job_record


def _wait_for_job(root, seconds: float = 30.0) -> None:
    import time

    store = Path(root) / ".factory" / "local" / "jobs"
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        for result_path in store.glob("*.result.json"):
            return
        time.sleep(0.2)
    raise AssertionError("the job wrote no result")


def _job_record(root) -> dict:
    store = Path(root) / ".factory" / "local" / "jobs"
    candidates = [
        json.loads(path.read_text())
        for path in sorted(store.glob("*.json"))
        if len(path.stem) == 16
    ]
    jobs_ = [one for one in candidates if one.get("kind") == "job"]
    assert jobs_
    return jobs_[-1]


def _survey_again(root) -> str:
    from tests.unit import story_ready
    import io

    from vsdd.cli import dispatch

    key = story_ready.write_caller(root)
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(
        ["-C", str(root), "survey", "story", "--subject", STORY, "--json"],
        stdout=out,
        stderr=err,
        stdin_tty=True,
        environ={"HOME": str(Path(root).parent), "VSDD_CALLER": key},
    )
    assert code == 0, err.getvalue()
    return json.loads(out.getvalue())["run_id"]


def test_through_launcher(tmp_path):
    """Exit codes through py/launch.sh itself: the launcher's cli entry is
    silent and speaks in codes — 1 for the gate's negative answer, 0 for its
    positive one."""
    import os
    import subprocess

    root, run = gate_project.make_project(tmp_path)
    gate_project.red_commit(root)
    env = gate_project.env_with(root, IMPL)
    launch = gate_project.NEXT / "plugins" / "vsdd-factory" / "py" / "launch.sh"

    def launch_code() -> int:
        environ = dict(os.environ)
        environ.update(env)
        environ["CLAUDE_PROJECT_DIR"] = str(root)
        done = subprocess.run(
            ["sh", str(launch), "cli", "-C", str(root),
             "gate", "demo", "--story", STORY, "--run", run, "--json"],
            cwd=root, capture_output=True, text=True, env=environ, timeout=300,
        )
        return done.returncode

    assert launch_code() == 1  # no green evidence yet: the negative answer is exit 1
    # the red gate passes, the source is fixed, green passes, demo holds: exit 0
    code_r, red, _err = run_gate(root, "gate", "red", "--story", STORY, "--run", run, env=env)
    assert code_r == 0 and red["passed"] is True, red["failures"]
    gate_project.commit_story(root, {"tally/blade.py": gate_project.BLADE_FIXED}, "the fix")
    code_g, green, _err = run_gate(root, "gate", "green", "--story", STORY, "--run", run, env=env)
    assert code_g == 0 and green["passed"] is True, green["failures"]
    assert launch_code() == 0
