"""Shared machinery for the runner's tests (plan 3.7).

The hook runner's tests do not need the state design's fixture factories: a
hook needs ``factory.toml`` and nothing else, so :func:`make_project` writes
only that. A test that needs real state starts from
``tests.unit._repo.make_project`` (S1's), which runs the real ``init``.

:func:`launch_copy` copies the plugin's ``py/`` tree, injects fixture rule
entries at the ``# <<< rules`` anchor of the copied registry, and returns the
copied launcher, so the launcher tests run the real shell under sh, bash and
dash with the real uv.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from pathlib import Path

from vsdd.hooks import main as hook_main
from vsdd.hooks.model import RuleEntry

NEXT = Path(__file__).resolve().parents[2]
PLUGIN = NEXT / "plugins" / "vsdd-factory"
PAYLOADS = Path(__file__).resolve().parent / "payloads"

PROBE_TABLE = Path(__file__).resolve().parent / "fixture_cases" / "probe.toml"


def make_project(tmp: Path, *, story: str | None = None, branch: str | None = None) -> Path:
    """One factory project: ``factory.toml`` and, when asked, a story or spike
    worktree (a name starting ``spike-``) whose ``.git`` is a linked-worktree
    pointer with the branch checked out — file-backed, so ``vsdd.project``
    classifies it without a process."""
    root = Path(tmp) / "factory-project"
    (root / ".factory").mkdir(parents=True, exist_ok=True)
    (root / ".factory" / "factory.toml").write_text("schema = 1\n", encoding="utf-8")
    if story is not None:
        if branch is None:
            if story.startswith("spike-"):
                branch = f"spike/{story[len('spike-'):]}"
            else:
                branch = f"story/{story}"
        _linked_worktree(root, story, branch)
    return root


def _linked_worktree(root: Path, name: str, branch: str) -> None:
    """A minimal linked worktree: ``HEAD`` through a ``gitdir`` pointer file."""
    worktree = root / ".worktrees" / name
    worktree.mkdir(parents=True, exist_ok=True)
    git_dir = root / ".git" / "worktrees" / name
    git_dir.mkdir(parents=True, exist_ok=True)
    (worktree / ".git").write_text(f"gitdir: {git_dir}\n", encoding="utf-8")
    head = f"ref: {branch}\n" if branch else "0" * 40 + "\n"
    (git_dir / "HEAD").write_text(head, encoding="utf-8")


def payload(name: str, **changes) -> dict:
    """One recorded payload, with dotted ``changes`` applied
    (``set = {"agent_type" = ...}`` in the case tables). The name is relative
    to ``payloads/``, with or without ``.json``."""
    if not name.endswith(".json"):
        name += ".json"
    text = (PAYLOADS / name).read_text(encoding="utf-8")
    value = json.loads(text)
    for dotted, replacement in changes.items():
        target = value
        parts = dotted.split(".")
        for part in parts[:-1]:
            target = target[part]
        target[parts[-1]] = replacement
    return value


def run_hook(payload_value, mode: str, *, entries=None, env: dict | None = None,
             stdout=None, stderr=None) -> tuple[int, str, str]:
    """One hook call in process; returns (code, stdout, stderr)."""
    import io

    out, err = io.StringIO(), io.StringIO()
    environment = os.environ.copy()
    environment["PYTHONDONTWRITEBYTECODE"] = "1"
    if env:
        environment.update(env)
    text = payload_value if isinstance(payload_value, str) else json.dumps(payload_value)
    code = hook_main.run(mode, text, environment, entries=entries, stdout=stdout or out, stderr=stderr or err)
    return code, (stdout or out).getvalue(), (stderr or err).getvalue()


def launch_copy(tmp: Path, rules: list[tuple[dict, str]] | None = None) -> tuple[Path, Path]:
    """Copy the plugin's ``py/`` tree into ``tmp/plugin`` and inject rule rows.

    ``rules`` pairs a row dictionary (the ``RuleEntry`` fields, with ``module``
    defaulting to ``vsdd.rules.<id>``) with that rule module's source; the
    module is written under the copy's ``vsdd/rules/`` and the row lands at
    the ``# <<< rules`` anchor of the copied registry. Returns (launch.sh,
    plugin root).
    """
    plugin = Path(tmp) / "plugin"
    shutil.copytree(PLUGIN / "py", plugin / "py")
    # The runner reads the shipped schemas beside the py tree (the event
    # schema is loaded at import), so the copy carries them too.
    shutil.copytree(PLUGIN / "schemas", plugin / "schemas")
    registry_copy = plugin / "py" / "vsdd" / "hooks" / "registry.py"
    text = registry_copy.read_text(encoding="utf-8")
    anchor = "# <<< rules"
    rows = []
    for row, source in rules or []:
        module_name = row.get("module", "vsdd.rules." + row["id"].replace("-", "_"))
        if source is not None:
            leaf = module_name.rsplit(".", 1)[-1]
            (plugin / "py" / "vsdd" / "rules" / f"{leaf}.py").write_text(source, encoding="utf-8")
        # The row lands whole, module key included: an injected row may point
        # at a module whose name the id does not imply.
        entry = ", ".join(f"{key}={value!r}" for key, value in row.items())
        rows.append(f"    RuleEntry({entry}),")
    inserted = text.replace(anchor, "\n".join(rows) + ("\n    " if rows else "") + anchor, 1)
    registry_copy.write_text(inserted, encoding="utf-8")
    launch = plugin / "py" / "launch.sh"
    launch.chmod(0o755)
    return launch, plugin


def launch(interpreter: str, launch_sh: Path, argv: list[str], cwd: Path, *,
           stdin_text: str = "", env: dict | None = None) -> subprocess.CompletedProcess:
    """Run one launcher under one interpreter, real uv included."""
    environment = os.environ.copy()
    environment["PYTHONDONTWRITEBYTECODE"] = "1"
    environment.setdefault("VSDD_EVENTS_DIR", str(Path(cwd) / ".factory" / "local" / "events"))
    if env:
        environment.update(env)
    return subprocess.run(
        [interpreter, str(launch_sh), *argv],
        cwd=Path(cwd),
        input=stdin_text,
        capture_output=True,
        text=True,
        env=environment,
        timeout=120,
    )


def entries_for(*rules: tuple[str, str, dict]) -> tuple[RuleEntry, ...]:
    """Helper for tests: (id, module suffix, overrides) to a RuleEntry tuple."""
    built = []
    for identifier, module, overrides in rules:
        fields = dict(
            id=identifier,
            on=(("PreToolUse", ("Bash",)),),
            kind="gate",
            fail="closed",
            module=module,
        )
        fields.update(overrides)
        built.append(RuleEntry(**fields))
    return tuple(built)


def root_markers(root: Path) -> list[Path]:
    return sorted((root / ".factory" / "local" / "hooks" / "calls").glob("*.json"))
