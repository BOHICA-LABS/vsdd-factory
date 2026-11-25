"""The vsdd.state facade (state-core.md 3.2): flat names resolve lazily and
loudly, no module shadows an exported name, and import loads no submodule."""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

import pytest

PACKAGE = Path(__file__).resolve().parents[2] / "plugins" / "vsdd-factory" / "py" / "vsdd" / "state"
API_SPEC = (
    Path(__file__).resolve().parents[3] / "rewrite" / "spec" / "topics" / "state-api.md"
)


def test_flat_name_resolves_to_owner():
    import vsdd.state

    assert vsdd.state.detect == pytest.importorskip("vsdd.state.mount").detect


def test_no_owner_or_two_is_loud(monkeypatch, tmp_path):
    """A name with no owner, or two, names the modules in the error."""
    stub = tmp_path / "sd_stub"
    stub.mkdir()
    (stub / "__init__.py").write_text(
        (PACKAGE / "__init__.py").read_text(encoding="utf-8"), encoding="utf-8"
    )
    (stub / "a.py").write_text("__all__ = ['collides']\ncollides = 1\n", encoding="utf-8")
    (stub / "b.py").write_text("__all__ = ['collides']\ncollides = 2\n", encoding="utf-8")
    (stub / "c.py").write_text("__all__ = ['absent']\n", encoding="utf-8")
    program = (
        "import sys; sys.path.insert(0, {root!r})\n"
        "import sd_stub as s\n"
        "try:\n    s.nobody_at_home\n"
        "except AttributeError as exc:\n    print('MISSING:', exc)\n"
        "try:\n    s.absent\n"
        "except AttributeError as exc:\n    print('UNDEFINED:', exc)\n"
        "try:\n    s.collides\n"
        "except AttributeError as exc:\n    print('DUP:', exc)\n"
    ).format(root=str(tmp_path))
    proc = subprocess.run(
        [sys.executable, "-I", "-B", "-c", program], capture_output=True, text=True
    )
    assert proc.returncode == 0, proc.stderr
    assert "MISSING:" in proc.stdout and "nobody_at_home" in proc.stdout
    assert "UNDEFINED:" in proc.stdout and "c.py" in proc.stdout
    assert "DUP:" in proc.stdout
    assert "a.py" in proc.stdout and "b.py" in proc.stdout


def test_no_module_shadows_an_exported_name():
    """``flush`` cannot live in flush.py: Python would bind the submodule."""
    modules = [p.stem for p in PACKAGE.glob("*.py")]
    for path in PACKAGE.glob("*.py"):
        if path.stem.startswith("_") or path.stem.startswith("cmd_"):
            continue
        block = re.search(r"^__all__\s*=\s*\[(.*?)\]", path.read_text(encoding="utf-8"), re.M | re.S)
        if not block:
            continue
        names = re.findall(r"\"([^\"]*)\"|'([^']*)'", block.group(1))
        exported = {first or second for first, second in names}
        clash = exported & set(modules)
        assert not clash, (path.name, clash)


def test_flat_names_are_state_api_names():
    text = API_SPEC.read_text(encoding="utf-8")
    api_names = set(re.findall(r"\b([a-z_][a-z_]*)\(", text)) | set(
        re.findall(r"`([a-z_]+)`,", text)
    )
    for path in sorted(PACKAGE.glob("*.py")):
        if path.stem.startswith("_") or path.stem.startswith("cmd_"):
            continue
        block = re.search(r"^__all__\s*=\s*\[(.*?)\]", path.read_text(encoding="utf-8"), re.M | re.S)
        if not block:
            continue
        names = re.findall(r"\"([^\"]*)\"|'([^']*)'", block.group(1))
        exported = {first or second for first, second in names}
        unknown = exported - api_names
        assert not unknown, (path.name, unknown, sorted(api_names))


def test_import_loads_no_submodule():
    program = "\n".join(
        [
            "import pathlib, sys",
            "sys.path.insert(0, {py!r})",
            "import vsdd.state",
            "loaded = [m for m in sys.modules if m.startswith('vsdd.state.')]",
            "assert loaded == [], loaded",
            "calls = []",
            "original = pathlib.Path.read_text",
            "def counted(self, *a, **k):",
            "    calls.append(str(self))",
            "    return original(self, *a, **k)",
            "pathlib.Path.read_text = counted",
            "fn = vsdd.state.detect",
            "first = len(calls)",
            "after = sorted(m for m in sys.modules if m.startswith('vsdd.state.'))",
            "fn2 = vsdd.state.detect",
            "second = len(calls) - first",
            "print('RESULTS', first, second, '|'.join(after))",
        ]
    ).format(py=str(PACKAGE.parent.parent))
    proc = subprocess.run([sys.executable, "-I", "-B", "-c", program], capture_output=True, text=True)
    assert proc.returncode == 0, proc.stderr
    results = proc.stdout.split("RESULTS")[1].split()
    first, second, after = int(results[0]), int(results[1]), results[2]
    assert first > 0  # the first lookup built the table and read the owner
    assert second == 0  # a second lookup reads no file
    # The first lookup imported exactly the owner (mount) and what mount
    # itself imports (gitrun) — not every sibling (finding
    # S3R-state-core-13).
    assert after.split("|") == ["vsdd.state.gitrun", "vsdd.state.mount"], after
