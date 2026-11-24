"""Shared fixtures for the hook-subsystem tests: interpreters, a fake uv, project trees.

The launcher cases must behave identically under sh, bash and dash (hooks.md 2.14), so
every behavioural test resolves the three interpreters up front and runs the launcher
once per interpreter.
"""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

NEXT = Path(__file__).resolve().parents[2]
PLUGIN = NEXT / "plugins" / "vsdd-factory"
LAUNCH_SH = PLUGIN / "py" / "launch.sh"
FIND_ROOT_SH = PLUGIN / "py" / "find-root.sh"

_FALLBACKS = {"sh": "/bin/sh", "bash": "/bin/bash", "dash": "/bin/dash"}


def _interpreters() -> dict[str, str]:
    found: dict[str, str] = {}
    for name in ("sh", "bash", "dash"):
        path = shutil.which(name) or _FALLBACKS[name]
        if Path(path).exists():
            found[name] = path
    return found


INTERPRETERS: dict[str, str] = _interpreters()

# The fake uv records every invocation (argv plus the VSDD_FACTORY_ROOT it received)
# into $VSDD_FAKE_UV_LOG, then exits with $VSDD_FAKE_UV_RC -- except "python find"
# subcommands, which exit with $VSDD_FAKE_UV_FIND_RC (default 0: a Python at the
# floor is present). That separation is what lets a test choose between uv's two
# distinct exit-2 causes.
FAKE_UV = """#!/bin/sh
printf '%s | uv0=%s | root=%s\\n' "$*" "$0" "${VSDD_FACTORY_ROOT-}" >> "${VSDD_FAKE_UV_LOG:?}"
case "$1 $2" in
  "python find") exit ${VSDD_FAKE_UV_FIND_RC:-0} ;;
  *) exit ${VSDD_FAKE_UV_RC:-0} ;;
esac
"""


def make_fake_uv(bin_dir: Path, log: Path) -> Path:
    bin_dir.mkdir(parents=True, exist_ok=True)
    uv = bin_dir / "uv"
    uv.write_text(FAKE_UV, encoding="utf-8")
    uv.chmod(0o755)
    return uv


def make_project(root: Path, factory: bool = True, legacy: bool = False) -> Path:
    base = root / ("factory-project" if factory else "plain-project")
    (base / ".factory").mkdir(parents=True, exist_ok=True)
    if factory:
        (base / ".factory" / "factory.toml").write_text("schema = 1\n", encoding="utf-8")
    if legacy:
        (base / ".factory" / "STATE.md").write_text("# pre-1.0 state\n", encoding="utf-8")
    return base


def launch(
    interpreter: str,
    argv: list[str],
    project: Path,
    *,
    path: str | None = None,
    home: Path | None = None,
    events_dir: Path | None = None,
    fake_log: Path | None = None,
    fake_rc: int | None = None,
    fake_find_rc: int | None = None,
    hooks_off: bool = False,
    selftest_no_uv: bool = False,
    cwd: str | os.PathLike[str] | None = None,
    extra_env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess:
    """Run py/launch.sh under `interpreter` with a controlled environment."""
    env = dict(os.environ)
    env["CLAUDE_PROJECT_DIR"] = str(project)
    env.pop("VSDD_FACTORY_HOOKS", None)
    env.pop("VSDD_EVENTS_DIR", None)
    if extra_env:
        env.update(extra_env)
    if path is not None:
        env["PATH"] = path
    if home is not None:
        env["HOME"] = str(home)
    if events_dir is not None:
        env["VSDD_EVENTS_DIR"] = str(events_dir)
    if fake_log is not None:
        env["VSDD_FAKE_UV_LOG"] = str(fake_log)
    if fake_rc is not None:
        env["VSDD_FAKE_UV_RC"] = str(fake_rc)
    if fake_find_rc is not None:
        env["VSDD_FAKE_UV_FIND_RC"] = str(fake_find_rc)
    if hooks_off:
        env["VSDD_FACTORY_HOOKS"] = "off"
    if selftest_no_uv:
        env["VSDD_FACTORY_SELFTEST_NO_UV"] = "1"
    return subprocess.run(
        [INTERPRETERS[interpreter], str(LAUNCH_SH)] + argv,
        input="{}",
        capture_output=True,
        text=True,
        timeout=120,
        env=env,
        cwd=str(cwd) if cwd is not None else str(project),
    )


def source_find_root(
    interpreter: str,
    project_dir: Path,
    home: Path | None = None,
    cwd: str | None = None,
) -> subprocess.CompletedProcess:
    """Source find-root.sh under `interpreter` and print the two resulting variables."""
    snippet = (
        f'. "{FIND_ROOT_SH}"; '
        'printf \'ROOT=%s\\nLEGACY=%s\\n\' "$FACTORY_ROOT" "$FACTORY_LEGACY"'
    )
    env = dict(os.environ)
    env["CLAUDE_PROJECT_DIR"] = str(project_dir)
    if home is not None:
        env["HOME"] = str(home)
    return subprocess.run(
        [INTERPRETERS[interpreter], "-c", snippet],
        capture_output=True,
        text=True,
        timeout=60,
        env=env,
        cwd=cwd or str(project_dir),
    )


def parsed_root(result: subprocess.CompletedProcess) -> tuple[str, str]:
    assert result.returncode == 0, f"sourcing find-root.sh failed: {result.stderr!r}"
    lines = result.stdout.splitlines()
    assert len(lines) == 2, f"unexpected output: {result.stdout!r}"
    root = lines[0].removeprefix("ROOT=")
    legacy = lines[1].removeprefix("LEGACY=")
    return root, legacy
