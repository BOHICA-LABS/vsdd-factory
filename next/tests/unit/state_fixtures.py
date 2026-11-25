"""The project builder the record-store tests start from (plan records.md 2).

``make_project(tmp_path)`` runs S1's real ``init`` (``_repo.make_project``)
and opens one cycle, the state every record-store test assumes.
``spec_file(root, rel, text)`` writes and commits one file under
``.factory/``, so a digest covers it.
"""

from __future__ import annotations

import subprocess
from pathlib import Path

from ._repo import make_project as _make_project
from vsdd.state import cycles

CYCLE = "product-1"


def make_project(tmp_path) -> Path:
    """A scratch project, real ``init``, one open cycle named ``product-1``."""
    root = _make_project(tmp_path)
    open_cycle(root)
    return root


def open_cycle(root: Path, cycle: str = CYCLE) -> dict:
    """One open product cycle, written by the real store."""
    return cycles.cycle_write(
        root,
        {
            "id": cycle,
            "kind": "product",
            "title": "Product cycle",
            "route": "standard",
            "opened": "2026-11-24T08:00:00Z",
            "baseline": "",
            "request": "brief",
        },
    )


def spec_file(root: Path, rel: str, text: str) -> Path:
    """Write one file into the state tree and commit it there, so the tracked
    state and any digest built on it see it. ``rel`` is project-relative
    (``.factory/specs/...``) or state-relative (``specs/...``)."""
    inner = rel[len(".factory/") :] if rel.startswith(".factory/") else rel
    path = Path(root) / ".factory" / inner
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    git(root / ".factory", "add", "-A", "--", inner)
    git(
        root / ".factory",
        "-c",
        "user.name=Factory Test",
        "-c",
        "user.email=factory@example.test",
        "commit",
        "-qm",
        f"factory: {inner}",
    )
    return path


def git(cwd: Path, *args: str) -> None:
    subprocess.run(["git", *args], cwd=str(cwd), check=True, capture_output=True)
