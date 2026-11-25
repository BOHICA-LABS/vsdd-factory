"""Through the launcher: py/launch.sh cli runs the survey and the checkpoint
end to end (plan survey.md section 7)."""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path

from . import story_ready
from .story_ready import make_project as make_story_project

STORY = "S-9.01"
LAUNCH_SH = Path(__file__).resolve().parents[2] / "plugins" / "vsdd-factory" / "py" / "launch.sh"


def _cli(root, *args: str, marker: str | None) -> subprocess.CompletedProcess:
    from pathlib import Path as _Path

    environ = {
        "HOME": str(_Path(root).parent),
        "PATH": os.environ.get("PATH", "/usr/bin:/bin"),
    }
    if marker is not None:
        environ["VSDD_CALLER"] = marker
    return subprocess.run(
        ["sh", str(LAUNCH_SH), "cli", *args],
        cwd=str(root),
        env=environ,
        capture_output=True,
        text=True,
        timeout=120,
    )


def test_cli_survey_then_checkpoint(tmp_path):
    root = make_story_project(tmp_path)
    marker = story_ready.write_caller(root)
    surveyed = _cli(root, "survey", "story", "--subject", STORY, "--json", marker=marker)
    assert surveyed.returncode == 0, surveyed.stderr
    survey = json.loads(surveyed.stdout)
    assert survey["ok"] is True and survey["units"][0]["id"] == STORY

    stopped = story_ready.write_caller(root)
    closed = _cli(
        root, "checkpoint", "story", "--run", survey["run_id"], "--json", marker=stopped
    )
    assert closed.returncode == 0, closed.stderr
    result = json.loads(closed.stdout)
    assert result["ok"] is True and result["status"] == "partial"
    from vsdd.state.runs import run_read

    assert run_read(root, survey["run_id"])["ended"]


def test_cli_survey_no_marker_exits_1_with_full_object(tmp_path):
    root = make_story_project(tmp_path)
    answered = _cli(root, "survey", "story", "--subject", STORY, "--json", marker=None)
    assert answered.returncode == 1
    data = json.loads(answered.stdout)
    assert data["ok"] is False
    assert "no caller marker" in data["error"]
    # Every field of the survey schema is printed, even here.
    for field in ("run_id", "units", "preconditions", "config", "limits", "next"):
        assert field in data
