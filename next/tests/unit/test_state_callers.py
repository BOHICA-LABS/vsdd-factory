"""The caller marker reader (plan records.md 7, test_caller): read once,
deleted, cached per root and key; old ones pruned; no key or file is None."""

from __future__ import annotations

import json
import os
import time

import pytest

from .state_fixtures import make_project
from vsdd.state import callers

KEY = "abcd1234abcd1234abcd1234abcd1234"


def _marker(root, key=KEY, session="sess-9", agent="ag-1", agent_type="builder", mtime=None):
    directory = root / ".factory" / "local" / "hooks" / "calls"
    directory.mkdir(parents=True, exist_ok=True)
    payload = {
        "v": 1,
        "key": key,
        "session_id": session,
        "agent_id": agent,
        "agent_type": agent_type,
        "cwd": str(root),
        "ts": "2026-11-24T12:00:00.000Z",
    }
    path = directory / f"{key}.json"
    path.write_text(json.dumps(payload), encoding="utf-8")
    if mtime is not None:
        os.utime(path, (mtime, mtime))
    return path


def test_caller(tmp_path):
    root = make_project(tmp_path)
    path = _marker(root)
    answer = callers.caller(KEY, project=root)
    assert answer == {
        "session_id": "sess-9",
        "agent_id": "ag-1",
        "agent_type": "builder",
        "cwd": str(root),
    }
    # Deleted on read…
    assert not path.exists()
    # …and the answer is cached per (root, key): the second read is the same.
    assert callers.caller(KEY, project=root) == answer

    # No file, no key, a short key: None.
    assert callers.caller("zzzz9999zzzz9999zzzz9999zzzz9999", project=root) is None
    assert callers.caller("", project=root) is None
    assert callers.caller("tiny", project=root) is None

    # A corrupted marker without a session_id is loud.
    key2 = "eeee1234eeee1234eeee1234eeee1234"
    directory = root / ".factory" / "local" / "hooks" / "calls"
    (directory / f"{key2}.json").write_text('{"v": 1, "agent_id": "x"}', encoding="utf-8")
    from vsdd.state.store import StoreCorrupt

    with pytest.raises(StoreCorrupt):
        callers.caller(key2, project=root)

    # Markers older than ten minutes are pruned by a later read.
    stale = _marker(root, key="eeee0000eeee0000eeee0000eeee0000", session="old", mtime=time.time() - 700)
    fresh = _marker(root, key="ffff0000ffff0000ffff0000ffff0000", session="new", mtime=time.time())
    assert stale.exists()
    callers.caller("aaaa0000aaaa0000aaaa0000aaaa0000", project=root)  # any read prunes
    assert not stale.exists()
    assert fresh.exists()


def test_cached_per_root(tmp_path):
    first = make_project(tmp_path / "one")
    second = make_project(tmp_path / "two")
    _marker(first)
    _marker(second, session="other")
    assert callers.caller(KEY, project=first)["session_id"] == "sess-9"
    assert callers.caller(KEY, project=second)["session_id"] == "other"
