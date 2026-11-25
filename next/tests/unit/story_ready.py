"""Builds a story-ready project: the sample every survey and checkpoint test
starts from (plan survey.md section 7; step-4 ORDER.md section 5, R1).

Everything is written through the real init (``_repo.make_project``), the
stores and the loader — never hand-written record JSON. Init leaves
``[commands]`` empty, so the helper sets ``commands.test`` and
``commands.test_report``, sets ``delivery.mode = local`` (the slice runs
local; the pr test builds its own remote), and flushes, so a survey's
opening flush starts from ``clean``.

``product`` is a directory or a callable: its contents are copied into the
project, ``[paths]`` is set, and the product is committed on the integration
branch — before the story is admitted, so the sample product is in the
story's base (R1). When E1's ``story-ready`` loader exists this becomes a
call to it.
"""

from __future__ import annotations

import io
import secrets
import shutil
import subprocess
import tomllib
from pathlib import Path

from vsdd.cli import dispatch

from ._repo import make_project as _make_project
from ._repo import set_keys

SESSION = "sess-1234-5678"


def make_project(tmp_path, *, product=None, story: str = "S-9.01", cycle: str = "c-1") -> Path:
    """A project ready for ``vsdd survey story --subject <story>``."""
    root = _make_project(Path(tmp_path), remote=False)
    set_keys(root, "commands", test="python -m pytest -q", test_report="report.xml")
    set_keys(root, "delivery", mode="local")
    _cycle(root, cycle)
    _approve_spec(root, cycle)
    _close_plan(root, cycle, [story])
    _story_file(root, story, cycle=cycle)
    if product is not None:
        _install_product(root, product)
    from vsdd.state.commits import flush

    result = flush(root, "story ready")
    if result["state"] not in ("committed", "clean"):
        raise AssertionError(f"story_ready flush did not settle: {result}")
    return root


def _cycle(root: Path, cycle: str) -> None:
    from vsdd.state.cycles import cycle_write
    from vsdd.state.store import now

    cycle_write(
        root,
        {
            "id": cycle,
            "kind": "product",
            "title": "Sample cycle",
            "route": "standard",
            "opened": now(),
            "baseline": "",
            "request": "brief",
        },
    )


def _approve_spec(root: Path, cycle: str) -> None:
    """The brief is approved (it opened the cycle) and the specification set
    is approved as it stands when the project is made."""
    from vsdd.state.decisions import decision_open, settle_decision
    from vsdd.state.digests import spec_digest

    decision_open(root, cycle=cycle, kind="brief", packet="# Brief approved")
    settle_decision(root, "brief", "approve", [], "", SESSION)

    digest, _files = spec_digest(root)
    decision_open(root, cycle=cycle, kind="spec", packet="# Specification approval", digest=digest)
    settle_decision(root, "spec", "approve", [], "", SESSION)


def _close_plan(root: Path, cycle: str, stories: list[str]) -> None:
    """The plan unit is closed: its run ended with the plan gate passed. The
    waves map is what admission reads (decision D4)."""
    from vsdd.state.cycles import closure_update

    def close(record: dict) -> None:
        record["state"] = "closed"
        record["waves"] = {"w1": list(stories)}

    closure_update(root, cycle, "plan", "", close)


def _story_file(root: Path, story: str, *, cycle: str) -> str:
    """A real story artifact the registry validates at flush. The registry
    derives the epic directory from the id: ``S-9.01`` lives in ``E-9``."""
    epic = f"E-{story[2:].split('.', 1)[0]}" if story.startswith("S-") else "E-1"
    rel = f".factory/stories/{epic}/{story}.md"
    path = root / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "\n".join(
            [
                "---",
                f"id: {story}",
                "title: Sample story",
                "status: draft",
                f"cycle: {cycle}",
                "kind: fix",
                "priority: P1",
                "criticality: medium",
                "contracts: [BC-1.00.001]",
                "demo: false",
                "---",
                "",
                "## Acceptance criteria",
                "",
                "### AC-1 (BC-1.00.001)",
                "",
                "The sample behaves as the contract says.",
                "",
            ]
        ),
        encoding="utf-8",
    )
    return rel


def _install_product(root: Path, product) -> None:
    """Copy a sample product in and commit it on the integration branch, so
    the story's base holds it (step-4 ORDER.md section 5, R1).

    The product's ``.factory`` and ``.git`` are never copied: the project
    keeps the configuration ``vsdd init`` gave it. When the product carries
    ``.factory/factory.toml``, its ``[paths]`` table is applied to the
    project — the table describes the layout the copy recreates — and
    nothing else is taken from that file, so ``delivery.mode`` and
    ``[state]`` stay the project's own. Without one, the source class is
    every copied top-level directory, never a file.
    """
    tops: list[str] = []
    if callable(product):
        product(root)
        tops = ["src"]
    else:
        source = Path(product)
        for child in sorted(source.iterdir()):
            if child.name in (".factory", ".git"):
                continue
            if child.is_dir():
                shutil.copytree(child, root / child.name, dirs_exist_ok=True)
                tops.append(child.name)
            else:
                shutil.copy2(child, root / child.name)
        product_config = source / ".factory" / "factory.toml"
        if product_config.is_file():
            paths = tomllib.loads(product_config.read_text(encoding="utf-8")).get("paths", {})
            set_keys(root, "paths", **paths)
        else:
            set_keys(root, "paths", source=[f"{name}/**" for name in tops])
    _git(root, "add", "-A")
    _git(root, "commit", "-qm", "sample product")


def _git(root: Path, *args: str) -> None:
    subprocess.run(["git", *args], cwd=root, check=True, capture_output=True, text=True)


# ------------------------------------------------------------------ helpers ---


def write_caller(root, *, session_id: str = SESSION) -> str:
    """Write one caller marker; the hook runner does this for real. Returns
    the key to pass as ``VSDD_CALLER``."""
    import json

    key = secrets.token_hex(8)
    directory = Path(root) / ".factory" / "local" / "hooks" / "calls"
    directory.mkdir(parents=True, exist_ok=True)
    (directory / f"{key}.json").write_text(
        json.dumps({"session_id": session_id, "agent_id": "", "agent_type": "", "cwd": str(root)}),
        encoding="utf-8",
    )
    return key


def run_vsdd(root, *args: str, environ=None) -> tuple[int, str, str]:
    """One in-process run of ``vsdd`` through S1's dispatcher."""
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(
        ["-C", str(root), *args],
        stdout=out,
        stderr=err,
        stdin_tty=True,
        environ=environ if environ is not None else {"HOME": str(root)},
    )
    return code, out.getvalue(), err.getvalue()
