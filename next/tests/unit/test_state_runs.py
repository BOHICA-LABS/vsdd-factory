"""The run record store (plan records.md 7, test_runs): a story recorded
once; ended, unknown and foreign story refuse; the first end time stays."""

from __future__ import annotations

import pytest

from .state_fixtures import make_project
from vsdd.state import runs
from vsdd.state.store import StoreRefused


def test_runs(tmp_path):
    root = make_project(tmp_path)
    run = runs.run_open(root, workflow="survey", cycle="product-1", session="sess-1")
    assert runs.run_live(root, run["id"])
    assert runs.run_read(root, run["id"]) is not None

    # A story recorded once; admitting it again changes nothing.
    runs.run_admit(root, run["id"], "S-9.01")
    runs.run_admit(root, run["id"], "S-9.01")
    stored = runs.run_read(root, run["id"])
    assert stored["stories"] == ["S-9.01"]
    assert stored["id"] == run["id"]

    # Membership is the one check every gate makes.
    runs.run_require(root, run["id"], story="S-9.01")
    with pytest.raises(StoreRefused) as exc:
        runs.run_require(root, run["id"], story="S-9.02")
    assert exc.value.code == "story_not_in_run"

    # Unknown runs refuse.
    with pytest.raises(StoreRefused) as exc:
        runs.run_require(root, "r-19700101-0000-xx")
    assert exc.value.code == "run_unknown"

    # Ending: the first end time stays.
    ended = runs.run_end(root, run["id"], now="2026-11-24T12:00:00Z")
    assert ended["ended"] == "2026-11-24T12:00:00Z"
    again = runs.run_end(root, run["id"], now="2026-11-24T13:00:00Z")
    assert again["ended"] == "2026-11-24T12:00:00Z"
    assert not runs.run_live(root, run["id"])

    # A live run is required; an ended run refuses the admit and the set.
    with pytest.raises(StoreRefused) as exc:
        runs.run_require(root, run["id"])
    assert exc.value.code == "run_ended"
    with pytest.raises(StoreRefused) as exc:
        runs.run_admit(root, run["id"], "S-9.03")
    assert exc.value.code == "run_ended"
    with pytest.raises(StoreRefused) as exc:
        runs.run_set(root, run["id"], agents=1)
    assert exc.value.code == "run_ended"

    # The end of an unknown run is nothing, not a refusal.
    assert runs.run_end(root, "r-19700101-0000-yy") is None

    # run_set carries the mutable fields; runs_list filters and orders.
    other = runs.run_open(root, workflow="spike", cycle="product-1", session="sess-2", now="2026-11-24T09:00:00Z")
    live = runs.runs_list(root, live=True)
    assert [r["id"] for r in live] == [other["id"]]
    assert [r["id"] for r in runs.runs_list(root, workflow="survey")] == [run["id"]]
    assert [r["id"] for r in runs.runs_list(root, subject="S-9.01")] == []


def test_run_life_and_amendment(tmp_path):
    root = make_project(tmp_path)
    run = runs.run_open(root, workflow="feature", cycle="product-1", session="s")
    updated = runs.run_set(root, run["id"], amendment=True, request="amend the contract")
    assert updated["amendment"] and updated["request"] == "amend the contract"
    child = runs.run_open(root, workflow="story", cycle="product-1", session="s", parent=run["id"])
    assert child["parent"] == run["id"]
    assert runs.runs_list(root, subject="S-9.01") == []


def test_concurrent_admits_from_real_processes(tmp_path):
    """Two processes admitting stories at once lose nothing: every
    read-modify-write holds flush.lock, so one admit cannot erase another
    (decision D1). The children are real processes: flock excludes across
    them, not across threads of one holder."""
    import subprocess
    import sys
    from pathlib import Path as _Path

    import vsdd

    py_dir = _Path(vsdd.__file__).resolve().parent.parent

    root = make_project(tmp_path)
    run = runs.run_open(root, workflow="story", cycle="product-1", session="s")

    children, expected = [], []
    program = (
        "import sys\n"
        "sys.path.insert(0, {py_dir!r})\n"
        "from vsdd.state import runs\n"
        "root, run_id, prefix, count = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])\n"
        "ready = root + '/.factory/local/ready-' + prefix\n"
        "open(ready, 'w').close()\n"
        "while True:\n"
        "    import os\n"
        "    if os.path.exists(root + '/.factory/local/go'):\n"
        "        break\n"
        "for i in range(count):\n"
        "    runs.run_admit(root, run_id, prefix + '-' + str(i))\n"
    ).format(py_dir=str(py_dir))

    go_flag = root / ".factory" / "local"
    go_flag.mkdir(parents=True, exist_ok=True)
    try:
        for index in range(4):
            prefix = f"S-{index}"
            expected.extend(f"{prefix}-{i}" for i in range(5))
            children.append(
                subprocess.Popen(
                    [sys.executable, "-I", "-B", "-c", program, str(root), run["id"], prefix, "5"],
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                )
            )
        # Everyone is ready; release the four at once.
        deadline = 40
        waited = 0
        while not all(
            (go_flag / f"ready-{n}").exists() for n in range(4)
        ):
            waited += 1
            if waited > deadline:
                break
        (go_flag / "go").write_text("", encoding="utf-8")
        for child in children:
            out, err = child.communicate(timeout=60)
            assert child.returncode == 0, err
        admitted = runs.run_read(root, run["id"])["stories"]
        assert sorted(admitted) == sorted(expected), "an admit was lost"
    finally:
        (go_flag / "go").write_text("", encoding="utf-8")
        for child in children:
            if child.poll() is None:
                child.kill()
