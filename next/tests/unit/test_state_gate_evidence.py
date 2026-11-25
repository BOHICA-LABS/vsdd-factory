"""The evidence store and lookup (plan records.md 7): a rewrite replaces and
the log is cut; an incomplete re-freeze refuses; the per-gate rules hold —
red over the branch, green by head and patch id, review from a converged
closure verdict, an unbuilt gate loud, a registered gate coming back."""

from __future__ import annotations

import re
import subprocess

import pytest

from .state_fixtures import make_project
from vsdd.state import cycles, gate_evidence, runs
from vsdd.state.store import StoreRefused


def _worktree(root):
    wt = root / ".worktrees" / "S-9.01"
    subprocess.run(
        ["git", "-C", str(root), "worktree", "add", "-b", "story/S-9.01", str(wt), "develop"],
        check=True,
        capture_output=True,
    )
    return wt


def _commit(wt: ..., message: str, content: str):
    (wt / "f.txt").write_text(content, encoding="utf-8")
    subprocess.run(["git", "-C", str(wt), "add", "-A"], check=True, capture_output=True)
    subprocess.run(
        ["git", "-C", str(wt), "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", message],
        check=True,
        capture_output=True,
    )
    return (
        subprocess.run(["git", "-C", str(wt), "rev-parse", "HEAD"], capture_output=True, text=True)
        .stdout.strip()
    )


def _prepared(tmp_path):
    root = make_project(tmp_path)
    wt = _worktree(root)
    base = (
        subprocess.run(["git", "-C", str(root), "rev-parse", "develop"], capture_output=True, text=True)
        .stdout.strip()
    )
    runs.run_open(root, workflow="story", cycle="product-1", session="s", subject="S-9.01")

    def set_base(record):
        record.setdefault("delivery", {})["base"] = base

    cycles.closure_update(root, "product-1", "story", "S-9.01", set_base)
    return root, wt, base


def test_evidence_write(tmp_path):
    root, _wt, base = _prepared(tmp_path)
    sha = "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b"
    record = {
        "gate": "red",
        "subject": "S-9.01",
        "sha": sha,
        "passed": True,
        "frozen": [{"file": "tests/x.py", "sha": "aa"}],
        "tests": ["tests/x.py::t"],
        "criteria": {"AC-1": ["tests/x.py::t"]},
    }
    written = gate_evidence.evidence_write(root, record, log="x" * 300_000 + "-tail")
    assert written["schema"] == 1 and written["at"]
    path = root / ".factory" / "local" / "evidence" / "S-9.01" / f"red.{sha[:12]}.json"
    assert path.is_file()
    # The log is the LAST 256 KB.
    log = (root / ".factory" / "local" / "evidence" / "S-9.01" / f"red.{sha[:12]}.log").read_text(
        encoding="utf-8"
    )
    assert len(log) == 256 * 1024 and log.endswith("-tail")

    # A rewrite replaces that gate and commit's record.
    gate_evidence.evidence_write(root, dict(record, passed=False), log="tiny")
    reread = gate_evidence.evidence_read(root, "S-9.01", "red", sha)
    assert reread["passed"] is False
    listing = [p.name for p in (root / ".factory" / "local" / "evidence" / "S-9.01").glob("*.json")]
    assert listing == [f"red.{sha[:12]}.json"]

    # A failing red record needs no frozen set, but an unknown gate refuses.
    with pytest.raises(StoreRefused) as exc:
        gate_evidence.evidence_write(root, {"gate": "nope", "subject": "S-9.01", "sha": sha, "passed": True})
    assert exc.value.code == "schema_invalid"


def test_refreeze_incomplete(tmp_path):
    root, _wt, base = _prepared(tmp_path)
    sha = "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b"
    first = {
        "gate": "red",
        "subject": "S-9.01",
        "sha": sha,
        "passed": True,
        "frozen": [{"file": "tests/a.py", "sha": "aa"}, {"file": "tests/b.py", "sha": "bb"}],
        "tests": ["tests/a.py::a", "tests/b.py::b"],
        "criteria": {"AC-1": ["tests/a.py::a"]},
    }
    gate_evidence.evidence_write(root, first)

    # A re-freeze that drops a frozen file refuses.
    incomplete = dict(first, refreeze=True, frozen=[{"file": "tests/a.py", "sha": "aa"}])
    with pytest.raises(StoreRefused) as exc:
        gate_evidence.evidence_write(root, incomplete)
    assert exc.value.code == "refreeze_incomplete"

    # The complete set: earlier identities and the changed hashes.
    complete = dict(
        first,
        refreeze=True,
        frozen=[{"file": "tests/a.py", "sha": "aa"}, {"file": "tests/b.py", "sha": "cc"}],
    )
    written = gate_evidence.evidence_write(root, complete)
    assert written["refreeze"] is True


def test_refreeze_chaining(tmp_path):
    """A re-freeze is judged against the latest earlier red that passed or
    re-froze — re-freezes included, failing reds skipped (probe: after
    refreeze(a,b,c), a refreeze(a,b) was accepted; after a failing red,
    refreeze([]) was accepted)."""
    root, _wt, base = _prepared(tmp_path)
    sha = "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b"
    red = {
        "gate": "red",
        "subject": "S-9.01",
        "sha": sha,
        "at": "2026-11-24T09:00:01Z",
        "passed": True,
        "frozen": [{"file": "tests/a.py", "sha": "aa"}, {"file": "tests/b.py", "sha": "bb"}],
        "tests": ["tests/a.py::a", "tests/b.py::b"],
        "criteria": {"AC-1": ["tests/a.py::a"]},
    }
    gate_evidence.evidence_write(root, red)

    # red(a,b), then refreeze(a,b,c): accepted, the set grows.
    grown = dict(red, refreeze=True, at="2026-11-24T09:00:02Z", frozen=[
        {"file": "tests/a.py", "sha": "aa"},
        {"file": "tests/b.py", "sha": "bb"},
        {"file": "tests/c.py", "sha": "cc"},
    ])
    gate_evidence.evidence_write(root, grown)

    # red(a,b), refreeze(a,b,c), then refreeze(a,b): refused — the latest
    # earlier re-freeze holds c, and skipping it loses the file.
    shrunk = dict(red, refreeze=True, at="2026-11-24T09:00:03Z", frozen=[
        {"file": "tests/a.py", "sha": "aa"}, {"file": "tests/b.py", "sha": "bb"},
    ])
    with pytest.raises(StoreRefused) as exc:
        gate_evidence.evidence_write(root, dict(shrunk, sha="b" + sha[1:]))
    assert exc.value.code == "refreeze_incomplete"
    assert "tests/c.py" in exc.value.args[0]

    # red(a,b), then a FAILING red, then refreeze([]): refused — the failing
    # red froze nothing; the last passing one froze a and b.
    gate_evidence.evidence_write(
        root, dict(red, sha="c" + sha[1:], at="2026-11-24T09:00:04Z", passed=False,
                   frozen=[], tests=[], criteria={})
    )
    with pytest.raises(StoreRefused) as exc:
        gate_evidence.evidence_write(
            root, dict(red, sha="d" + sha[1:], at="2026-11-24T09:00:05Z",
                       refreeze=True, frozen=[])
        )
    assert exc.value.code == "refreeze_incomplete"
    assert "tests/a.py" in exc.value.args[0]


def test_evidence_lookup(tmp_path):
    root, wt, base = _prepared(tmp_path)
    head = _commit(wt, "one", "one\n")
    red = {
        "gate": "red",
        "subject": "S-9.01",
        "sha": head,
        "passed": True,
        "frozen": [{"file": "tests/a.py", "sha": "aa"}],
        "tests": ["tests/a.py::a"],
        "criteria": {"AC-1": ["tests/a.py::a"]},
        "patch_id": gate_evidence.patch_id(wt, base, head),
    }
    gate_evidence.evidence_write(root, red)

    # Red after later commits: the record's commit is an ancestor of the head.
    head2 = _commit(wt, "two", "two\n")
    found = gate_evidence.evidence(root, "red", "S-9.01")
    assert found["sha"] == head

    # Green: none at the first head, then the head's record; after a commit
    # with no green, the patch id of a clean history still binds (empty diff
    # apart from the story's own commits, so no green matches).
    assert gate_evidence.evidence(root, "green", "S-9.01") is None
    gate_evidence.evidence_write(
        root, {"gate": "green", "subject": "S-9.01", "sha": head2, "passed": True}
    )
    found = gate_evidence.evidence(root, "green", "S-9.01")
    assert found["sha"] == head2

    # Red is not returned for a failing record: only passing or re-freeze.
    gate_evidence.evidence_write(root, dict(red, sha=head2, passed=False, frozen=[], tests=[], criteria={}))
    assert gate_evidence.evidence(root, "red", "S-9.01")["passed"] is True
    assert gate_evidence.evidence(root, "red", "S-9.01")["sha"] == head

    # Off-branch red: a record on develop is no ancestor of the head only if
    # the branch moved; a record whose commit is outside base..head is out.
    develop_head = _commit(root, "on-develop", "unrelated\n")
    gate_evidence.evidence_write(
        root, {"gate": "red", "subject": "S-9.01", "sha": develop_head, "passed": True,
               "frozen": [{"file": "tests/z.py", "sha": "zz"}],
               "tests": ["tests/z.py::z"], "criteria": {}}
    )
    found = gate_evidence.evidence(root, "red", "S-9.01")
    assert found["sha"] == head  # the story branch's record wins by recency rule

    # Review: the closure's verdict when converged and covering the head, by
    # the stored verdict's own head in summary()'s form (S4R-verdicts-1).
    def converge(record):
        record["verdict"] = {
            "outcome": "converged",
            "passes": 3,
            "head": head2,
            "patch_id": gate_evidence.patch_id(wt, base, head2),
        }

    cycles.closure_update(root, "product-1", "story", "S-9.01", converge)
    verdict = gate_evidence.evidence(root, "review", "S-9.01")
    assert verdict["outcome"] == "converged"
    # After a new commit the closure no longer covers the head.
    head3 = _commit(wt, "three", "three\n")
    assert gate_evidence.evidence(root, "review", "S-9.01") is None

    # An unbuilt gate is loud; a registered function comes back.
    with pytest.raises(StoreRefused) as exc:
        gate_evidence.evidence(root, "performance", "S-9.01")
    assert exc.value.code == "gate_lookup_not_built"
    assert gate_evidence.registered_gate("performance") is None

    sentinel = object()
    gate_evidence.register_gate("performance", sentinel)
    assert gate_evidence.registered_gate("performance") is sentinel
    del head3


def test_same_second_records_order_by_write(tmp_path):
    """Records written within the same whole-second keep their write order
    (S3B-records-1): each record carries a ``seq`` taken under the store's
    lock, one more than the highest in the subject's directory, and the
    lookup orders by it — the tie no longer falls to the file name."""
    root, _wt, _base = _prepared(tmp_path)
    at = "2026-11-24T10:00:00Z"
    name_sorts_last = "f" * 40  # its file name sorts last
    name_sorts_first = "0" * 40  # its file name sorts first
    red = {
        "gate": "red",
        "subject": "S-9.01",
        "sha": name_sorts_last,
        "at": at,
        "passed": True,
        "frozen": [{"file": "tests/a.py", "sha": "aa"}],
        "tests": ["tests/a.py::a"],
        "criteria": {"AC-1": ["tests/a.py::a"]},
    }
    gate_evidence.evidence_write(root, dict(red))
    gate_evidence.evidence_write(root, dict(red, sha=name_sorts_first))

    records = gate_evidence._evidence_records(root, "S-9.01", "red")
    # Write order, not name order: under an `at`-then-name sort the first
    # record would read as the latest.
    assert [record["sha"] for record in records] == [name_sorts_last, name_sorts_first]
    seqs = [record["seq"] for record in records]
    assert seqs == sorted(seqs) and min(seqs) >= 1

    # A rewrite numbers after what is there, so a replaced record is the
    # latest again.
    gate_evidence.evidence_write(
        root, dict(red, passed=False, frozen=[], tests=[], criteria={})
    )
    records = gate_evidence._evidence_records(root, "S-9.01", "red")
    assert [record["sha"] for record in records] == [name_sorts_first, name_sorts_last]
    assert records[-1]["seq"] > seqs[-1]


def test_evidence_without_worktree_or_base(tmp_path):
    root, _wt, base = _prepared(tmp_path)
    # No worktree for a subject that has none.
    assert gate_evidence.evidence(root, "red", "S-absent") is None
    # A subject with a worktree but no base in its closure.
    root2 = make_project(tmp_path / "second")
    _worktree(root2)
    # No run, no closure: no base.
    assert gate_evidence.evidence(root2, "red", "S-9.01") is None


def test_green_matches_by_patch_id_after_amend(tmp_path):
    """Green follows the patch id: after a commit is amended, the tree (and
    so the patch id) is unchanged, and the record still covers the head."""
    import subprocess

    root, wt, base = _prepared(tmp_path)
    head = _commit(wt, "one", "one\n")
    gate_evidence.evidence_write(
        root,
        {
            "gate": "green",
            "subject": "S-9.01",
            "sha": head,
            "passed": True,
            "patch_id": gate_evidence.patch_id(wt, base, head),
        },
    )
    # Amending leaves the tree identical; only the commit id moves.
    subprocess.run(
        ["git", "-C", str(wt), "-c", "user.name=t", "-c", "user.email=t@t",
         "commit", "--amend", "-q", "-m", "one, amended"],
        check=True, capture_output=True,
    )
    # The stored patch_id is the id alone — forty hex characters — not git's
    # whole output line with its zero commit id appended (S3R-records-26).
    assert re.fullmatch("[0-9a-f]{40}", gate_evidence.patch_id(wt, base, head))
    amended = (
        subprocess.run(["git", "-C", str(wt), "rev-parse", "HEAD"], capture_output=True, text=True)
        .stdout.strip()
    )
    assert amended != head
    found = gate_evidence.evidence(root, "green", "S-9.01")
    assert found is not None and found["sha"] == head
    assert gate_evidence.patch_id(wt, base, amended) == gate_evidence.patch_id(wt, base, head)


def test_red_before_base_is_rejected(tmp_path):
    """A red record on a commit before the base is not coverage, even though
    that commit is an ancestor of the head: the record must descend from the
    closure's delivery.base."""
    import subprocess

    root = make_project(tmp_path)
    # Give develop a second commit, so a commit exists before the base.
    _commit(root, "prep", "prep\n")
    wt = _worktree(root)
    base = (
        subprocess.run(["git", "-C", str(root), "rev-parse", "develop"], capture_output=True,
                       text=True)
        .stdout.strip()
    )
    runs.run_open(root, workflow="story", cycle="product-1", session="s", subject="S-9.01")

    def set_base(record):
        record.setdefault("delivery", {})["base"] = base

    cycles.closure_update(root, "product-1", "story", "S-9.01", set_base)
    _commit(wt, "one", "one\n")

    before_base = (
        subprocess.run(["git", "-C", str(root), "rev-parse", "develop~1"],
                       capture_output=True, text=True)
        .stdout.strip()
    )
    assert before_base and before_base != base
    gate_evidence.evidence_write(
        root,
        {
            "gate": "red",
            "subject": "S-9.01",
            "sha": before_base,
            "at": "2026-11-24T09:00:01Z",
            "passed": True,
            "frozen": [{"file": "tests/old.py", "sha": "aa"}],
            "tests": ["tests/old.py::o"],
            "criteria": {"AC-0": ["tests/old.py::o"]},
        },
    )
    found = gate_evidence.evidence(root, "red", "S-9.01")
    assert found is None, "a red record before the base is not coverage"

    # And the mirror: a record ON the base counts (the branch's start).
    base_record = {
        "gate": "red",
        "subject": "S-9.01",
        "sha": base,
        "at": "2026-11-24T09:00:02Z",
        "passed": True,
        "frozen": [{"file": "tests/base.py", "sha": "ab"}],
        "tests": ["tests/base.py::b"],
        "criteria": {"AC-1": ["tests/base.py::b"]},
    }
    gate_evidence.evidence_write(root, base_record)
    found = gate_evidence.evidence(root, "red", "S-9.01")
    assert found is not None and found["sha"] == base


def test_log_replace_failure_leaves_the_old_log(tmp_path, monkeypatch):
    """The .log goes through tmp-and-replace: a replace that fails leaves the
    old .log intact and no partial file beside the record
    (S3R-records-20)."""
    import os

    root, _wt, _base = _prepared(tmp_path)
    sha = "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b"
    record = {"gate": "green", "subject": "S-9.01", "sha": sha, "passed": True}
    gate_evidence.evidence_write(root, dict(record), log="old log text")
    log_path = root / ".factory" / "local" / "evidence" / "S-9.01" / f"green.{sha[:12]}.log"
    before = log_path.read_bytes()

    real_replace = os.replace

    def killing_replace(src, dst, *args, **kwargs):
        if str(dst).endswith(".log"):
            raise OSError("replace failed")
        return real_replace(src, dst, *args, **kwargs)

    monkeypatch.setattr(os, "replace", killing_replace)
    with pytest.raises(OSError):
        gate_evidence.evidence_write(root, dict(record), log="new log text")
    monkeypatch.undo()
    assert log_path.read_bytes() == before, "the old .log is intact"
    directory = log_path.parent
    assert not (directory / f"green.{sha[:12]}.tmp").exists(), "no partial file beside the record"
    tmp_dir = root / ".factory" / "local" / "tmp"
    if tmp_dir.is_dir():
        assert list(tmp_dir.iterdir()) == [], "the temporary file is cleaned up"


def test_evidence_drop_removes_the_directory(tmp_path):
    """``evidence_drop`` removes the subject's evidence directory, and a
    second call on what is gone is a no-op (plan records.md 3)."""
    root, _wt, _base = _prepared(tmp_path)
    sha = "1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b"
    gate_evidence.evidence_write(
        root, {"gate": "green", "subject": "S-9.01", "sha": sha, "passed": True}, log="text"
    )
    directory = gate_evidence.evidence_dir(root, "S-9.01")
    assert directory.is_dir()
    gate_evidence.evidence_drop(root, "S-9.01")
    assert not directory.exists()
    gate_evidence.evidence_drop(root, "S-9.01")  # nothing there: no raise


def test_gate_modules_are_discovered(tmp_path, monkeypatch):
    """The first lookup imports each vsdd.gates.gate_*.py, and a gate module
    that fails is loud: only a missing vsdd.gates is swallowed."""
    import sys
    import types

    package_dir = tmp_path / "vsdd" / "gates"
    package_dir.mkdir(parents=True)
    (package_dir / "__init__.py").write_text("", encoding="utf-8")
    (package_dir / "gate_probe.py").write_text(
        "from vsdd.state.gate_evidence import register_gate\n"
        "def measure(project, subject, run):\n"
        "    return {}\n"
        "register_gate('probe-gate', measure)\n",
        encoding="utf-8",
    )
    fake = types.ModuleType("vsdd.gates")
    fake.__path__ = [str(package_dir)]
    fake.__file__ = str(package_dir / "__init__.py")
    saved = {name: sys.modules.get(name) for name in ("vsdd.gates", "vsdd.gates.gate_probe")}
    monkeypatch.setitem(sys.modules, "vsdd.gates", fake)
    saved_flags = (gate_evidence._GATES_LOADED, gate_evidence._GATES_LOADING)
    gate_evidence._GATES_LOADED = False
    gate_evidence._GATES_LOADING = False
    try:
        # Discovery: the probe module registers at import.
        import vsdd.state.gate_evidence as ge

        importlib_reload_ge = ge
        found = gate_evidence.registered_gate("probe-gate")
        assert found is not None
        assert gate_evidence._GATES_LOADED is True
    finally:
        for name, module in saved.items():
            if module is None:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = module
        gate_evidence._GATES_LOADED, gate_evidence._GATES_LOADING = saved_flags
        sys.modules.pop("vsdd.gates.gate_broken", None)
        sys.modules.pop("vsdd.gates.gate_probe", None)


def test_gate_modules_discover_broken_gate_is_loud(tmp_path, monkeypatch):
    """A gate module that fails its import surfaces the failure on the
    lookup; it is not silently skipped and the loaded mark does not stick."""
    import sys
    import types

    package_dir = tmp_path / "vsdd" / "gates"
    package_dir.mkdir(parents=True)
    (package_dir / "__init__.py").write_text("", encoding="utf-8")
    (package_dir / "gate_boom.py").write_text("raise RuntimeError('boom')\n", encoding="utf-8")
    fake = types.ModuleType("vsdd.gates")
    fake.__path__ = [str(package_dir)]
    fake.__file__ = str(package_dir / "__init__.py")
    monkeypatch.setitem(sys.modules, "vsdd.gates", fake)
    saved_flags = (gate_evidence._GATES_LOADED, gate_evidence._GATES_LOADING)
    gate_evidence._GATES_LOADED = False
    gate_evidence._GATES_LOADING = False
    try:
        with pytest.raises(RuntimeError, match="boom"):
            gate_evidence.registered_gate("anything")
        assert gate_evidence._GATES_LOADED is False
    finally:
        gate_evidence._GATES_LOADED, gate_evidence._GATES_LOADING = saved_flags
        sys.modules.pop("vsdd.gates.gate_boom", None)


def test_gate_modules_missing_import_is_not_swallowed(tmp_path, monkeypatch):
    """Only a MISSING vsdd.gates package is swallowed: a shipped package whose
    gate module imports a module nobody has raises ModuleNotFoundError on the
    lookup — it does not read as 'no gates package' and return None."""
    import sys
    import types

    package_dir = tmp_path / "vsdd" / "gates"
    package_dir.mkdir(parents=True)
    (package_dir / "__init__.py").write_text("", encoding="utf-8")
    (package_dir / "gate_absent.py").write_text(
        "import definitely_not_a_module_vsdd_tests\n", encoding="utf-8"
    )
    fake = types.ModuleType("vsdd.gates")
    fake.__path__ = [str(package_dir)]
    fake.__file__ = str(package_dir / "__init__.py")
    monkeypatch.setitem(sys.modules, "vsdd.gates", fake)
    saved_flags = (gate_evidence._GATES_LOADED, gate_evidence._GATES_LOADING)
    gate_evidence._GATES_LOADED = False
    gate_evidence._GATES_LOADING = False
    try:
        with pytest.raises(ModuleNotFoundError):
            gate_evidence.registered_gate("anything")
        assert gate_evidence._GATES_LOADED is False
    finally:
        gate_evidence._GATES_LOADED, gate_evidence._GATES_LOADING = saved_flags
        sys.modules.pop("vsdd.gates.gate_absent", None)
