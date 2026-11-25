"""The store's foundation (plan records.md 7): refusals are loud, the file
lock excludes a second holder, and the facade publishes every state-API name.
"""

from __future__ import annotations

import json

import pytest

from vsdd.state import store
from vsdd.state.store import (
    StoreBusy,
    StoreCorrupt,
    StoreRefused,
    read_record,
    safe_segment,
    write_record,
)


def test_store_refusals(tmp_path):
    path = tmp_path / "record.json"
    write_record(path, {"a": 1}, cap=None)
    original = path.read_bytes()

    with pytest.raises(StoreRefused) as exc:
        write_record(path, {"a": "x" * 3000}, cap=2_000)
    assert exc.value.code == "over_cap"
    with pytest.raises(StoreRefused) as exc:
        write_record(path, {"nonsense": True}, cap=None, schema="cycle-record")
    assert exc.value.code == "schema_invalid"
    with pytest.raises(StoreRefused) as exc:
        safe_segment("../etc")
    assert exc.value.code == "unsafe_id"
    with pytest.raises(StoreRefused):
        safe_segment("a/b")
    with pytest.raises(StoreRefused):
        safe_segment("a..b")  # '..' anywhere is refused: the plan's 'without ..'
    assert safe_segment("S-9.01") == "S-9.01"
    assert safe_segment("_standing") == "_standing"
    assert safe_segment("_base") == "_base"  # step-4 request R2
    assert safe_segment("_") == "_"

    # The old bytes stay after every refused write.
    assert path.read_bytes() == original

    # A truncated record is corrupt, and the error names the path.
    path.write_text('{"a": ', encoding="utf-8")
    with pytest.raises(StoreCorrupt) as exc:
        read_record(path)
    assert str(path) in str(exc.value)
    assert read_record(tmp_path / "absent.json") is None

    # Local records are compact; tracked ones carry the indent.
    local = tmp_path / "local.json"
    write_record(local, {"a": 1}, cap=None)  # no .factory in the parts: indented
    tracked = tmp_path / ".factory" / "local" / "r.json"
    write_record(tracked, {"a": 1}, cap=None)
    assert b'{\n  "a": 1\n}' in local.read_bytes()
    assert json.loads(tracked.read_bytes()) == {"a": 1}
    assert b": " not in tracked.read_bytes()


def test_locked(tmp_path):
    from vsdd.state import store

    with store.locked(tmp_path, "flush.lock", wait=0.5):
        # Re-entrant in one process.
        with store.locked(tmp_path, "flush.lock", wait=0.5):
            pass
    # After release the lock is had again.
    with store.locked(tmp_path, "flush.lock", wait=0.5):
        pass


def test_second_process_gets_busy(tmp_path):
    """A second process gets StoreBusy, not a silent share (Darwin's flock is
    per-process, so the other holder is a real subprocess)."""
    import subprocess
    import sys
    import time
    (tmp_path / ".factory" / "local").mkdir(parents=True, exist_ok=True)
    holder = subprocess.Popen(
        [
            sys.executable,
            "-I",
            "-c",
            "import fcntl, os, time\n"
            f"fd = os.open({str(tmp_path / '.factory' / 'local' / 'flush.lock')!r}, os.O_RDWR | os.O_CREAT)\n"
            "fcntl.flock(fd, fcntl.LOCK_EX)\n"
            "print('held', flush=True)\n"
            "time.sleep(5)\n",
        ],
        stdout=subprocess.PIPE,
        text=True,
    )
    try:
        assert holder.stdout is not None
        line = holder.stdout.readline()
        assert line.strip() == "held"
        started = time.monotonic()
        with pytest.raises(StoreBusy):
            with store.locked(tmp_path, "flush.lock", wait=0.3):
                pass
        assert time.monotonic() - started < 2
    finally:
        holder.kill()
        holder.wait()


def test_error_tree():
    """Only refusals subclass StoreRefused: StoreBusy and StoreCorrupt are
    StoreErrors of their own, so a consumer catching StoreRefused does not
    swallow a busy lock or a corrupt file as an ordinary refusal."""
    assert issubclass(StoreRefused, store.StoreError)
    assert issubclass(store.DecisionRefused, StoreRefused)
    assert issubclass(StoreBusy, store.StoreError)
    assert not issubclass(StoreBusy, StoreRefused)
    assert issubclass(StoreCorrupt, store.StoreError)
    assert not issubclass(StoreCorrupt, StoreRefused)


def test_compact_only_under_factory_local(tmp_path):
    """Compact output is chosen for paths under ``.factory/local/`` — the
    component that follows ``.factory`` decides, not any ``local`` part."""
    # A project root named "local": its tracked records stay indented even
    # though the path carries a "local" part.
    tricky = tmp_path / "local"
    tracked = tricky / ".factory" / "cycles" / "c" / "cycle.json"
    write_record(tracked, {"a": 1}, cap=None)
    assert tracked.read_bytes().startswith(b'{\n  "a": 1')

    # A path under .factory/local/ is compact.
    loose = tricky / ".factory" / "local" / "runs" / "r.json"
    write_record(loose, {"a": 1}, cap=None)
    assert loose.read_bytes() == b'{"a":1}'


def test_package_names():
    """Each state-API name resolves through the facade to its own module."""
    import vsdd.state

    expected = {
        "store_step": "vsdd.state.steps",
        "evidence": "vsdd.state.gate_evidence",
        "digest": "vsdd.state.digests",
        "settle_decision": "vsdd.state.decisions",
        "caller": "vsdd.state.callers",
        "lock_seen": "vsdd.state.locks",
        "lock_release": "vsdd.state.locks",
        "flush": "vsdd.state.commits",
    }
    for name, module in expected.items():
        value = getattr(vsdd.state, name)
        owner = f"{module}.{name}"
        assert getattr(value, "__module__", "").startswith(module), (name, owner)
