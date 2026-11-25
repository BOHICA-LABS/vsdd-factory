"""The project builder every unit's tests start from (state-core.md 2).

``make_project(tmp_path, *, remote=False)`` builds a scratch repository with
one commit on ``develop`` and then runs the real ``vsdd init``, in process.
``set_keys(root, section, **values)`` rewrites keys of its ``factory.toml``.
"""

from __future__ import annotations

import io
import json
import subprocess
import tomllib
from pathlib import Path

from vsdd.cli import dispatch

IDENTITY = ("git", ["-c", "user.email=factory@example.test", "-c", "user.name=Factory Test"])


def _git(root: Path, *args: str) -> None:
    subprocess.run(
        ["git", *args],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )


def _toml_value(value) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, str):
        return json.dumps(value)
    if isinstance(value, (int, float)):
        return repr(value)
    if isinstance(value, list):
        return "[" + ", ".join(_toml_value(item) for item in value) + "]"
    if isinstance(value, dict):
        return "{ " + ", ".join(f"{k} = {_toml_value(v)}" for k, v in value.items()) + " }"
    raise TypeError(type(value).__name__)


def _dump(data: dict, prefix: str = "") -> list[str]:
    lines: list[str] = []
    tables: list[tuple[str, dict]] = []
    for key, value in data.items():
        if isinstance(value, dict):
            tables.append((key, value))
        else:
            lines.append(f"{key} = {_toml_value(value)}")
    for key, value in data.items():
        if not isinstance(value, dict):
            continue
        name = f"{prefix}.{key}" if prefix else key
        lines.append("")
        if all(not isinstance(inner, dict) for inner in value.values()):
            lines.append(f"[{name}]")
            for inner_key, inner in value.items():
                lines.append(f"{inner_key} = {_toml_value(inner)}")
        else:
            lines.append(f"[{name}]")
            for inner_key, inner in value.items():
                if not isinstance(inner, dict):
                    lines.append(f"{inner_key} = {_toml_value(inner)}")
            for inner_key, inner in value.items():
                if isinstance(inner, dict):
                    lines.extend(_dump({inner_key: inner}, prefix=name))
    return lines


def set_keys(root, section: str, **values) -> None:
    """Rewrite keys of ``<root>/.factory/factory.toml``."""
    path = Path(root) / ".factory" / "factory.toml"
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    target = data
    if section:
        target = data.setdefault(section, {})
    target.update(values)
    path.write_text("\n".join(_dump(data)) + "\n", encoding="utf-8")


def run_init(root, *args: str, environ=None) -> int:
    """The real init, in process, quietly."""
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(
        ["-C", str(root), *args],
        stdout=out,
        stderr=err,
        stdin_tty=True,
        environ=environ if environ is not None else {"HOME": str(root)},
    )
    if code != 0:
        raise AssertionError(f"make_project init failed ({code}): {err.getvalue()}")
    return code


def make_project(tmp_path, *, remote: bool = False) -> Path:
    """A scratch repository with one commit on ``develop`` and real init."""
    root = Path(tmp_path) / "proj"
    root.mkdir(parents=True, exist_ok=True)
    _git(root, "init", "-q", "-b", "develop")
    _git(root, "config", "user.email", "factory@example.test")
    _git(root, "config", "user.name", "Factory Test")
    (root / "README.md").write_text("# scratch\n", encoding="utf-8")
    (root / "src").mkdir()
    (root / "src" / "module.py").write_text("value = 1\n", encoding="utf-8")
    (root / "tests").mkdir()
    (root / "tests" / "test_module.py").write_text(
        "def test_module():\n    assert True\n", encoding="utf-8"
    )
    _git(root, "add", "-A")
    _git(root, "commit", "-qm", "initial")
    if remote:
        bare = Path(tmp_path) / "remote.git"
        subprocess.run(
            ["git", "init", "-q", "--bare", "-b", "develop", str(bare)],
            check=True,
            capture_output=True,
        )
        _git(root, "remote", "add", "origin", str(bare))
        _git(root, "push", "-q", "-u", "origin", "develop")
    run_init(root, "init")
    return root
