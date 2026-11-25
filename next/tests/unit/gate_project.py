"""The tally sample with one admitted feature story: the one sample-based
builder of the gate tests (plan gates.md section 7; step-4 ORDER.md section 5,
R1). Built on ``story_ready(product=...)``.

The product is the e2e tally sample itself: ``story_ready`` copies it without
its ``.factory``, takes the sample's own ``[paths]`` from
``.factory/factory.toml``, and commits it on the integration branch before
the story is admitted, so the tally tests are in the story's base and the
base report is non-empty. ``make_project`` runs the real survey (which opens
the run and admits the story) and hands the caller the story worktree, where
``commit_story`` adds the story's own commits.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

from . import story_ready
from ._repo import set_keys

NEXT = Path(__file__).resolve().parents[2]
SAMPLE = NEXT / "tests" / "e2e" / "sample-project"
STORY = "S-9.01"

TEST_WRITER = "vsdd-factory:test-writer"
IMPLEMENTER = "vsdd-factory:implementer"

NEW_TESTS = '''"""The story's new tests: red first (failing on assertions), green after
the source gains the behaviour. This file is frozen at the red gate: the
green commits change source only."""

import unittest

from tally.blade import sharpen


class TestSharpen(unittest.TestCase):
    def test_sharpens_by_one(self):
        self.assertEqual(sharpen(1), 2)

    def test_sharpen_zero(self):
        self.assertEqual(sharpen(0), 1)

    def test_ac1_sharpen_is_linenoise(self):
        self.assertEqual(sharpen(3), 4)


if __name__ == "__main__":
    unittest.main()
'''

BLADE_WITH_STUB = '''"""The story's source: a stub while the behaviour is written."""

# VSDD-STUB: the behaviour is not written yet


def sharpen(teeth: int) -> int:
    return teeth
'''

BLADE_FIXED = '''"""The story's source: the behaviour, written."""


def sharpen(teeth: int) -> int:
    return teeth + 1
'''


def make_project(tmp_path, *, story: str = STORY, test_cmd: str | None = None):
    """A project whose story is admitted: ``root, run``. ``commands.test``
    runs the sample's junit runner — the one the gate reads reports from."""
    root = story_ready.make_project(tmp_path, product=SAMPLE, story=story)
    if test_cmd is None:
        test_cmd = f"{sys.executable} tools/junit_runner.py"
    set_keys(root, "commands", test=test_cmd, test_report="junit:build/junit.xml",
             build="", lint="", format_check="", ui_check="", docs_check="")
    _assert_sample_configuration(root)
    key = _write_caller(root, IMPLEMENTER, story_ready.SESSION)
    env = {"HOME": str(Path(tmp_path).parent), "VSDD_CALLER": key}
    code, out, err = story_ready.run_vsdd(
        root, "survey", "story", "--subject", story, "--json", environ=env
    )
    if code != 0:
        raise AssertionError(f"survey failed ({code}): {err} {out}")
    run = json.loads(out).get("run_id", "")
    if not run:
        raise AssertionError(f"the survey opened no run: {out}")
    return root, run


def _assert_sample_configuration(root) -> None:
    """The project the gates are tested against is the sample as the sample's
    own ``.factory/factory.toml`` describes it (finding ORCH-5). A regression
    of ``story_ready``'s product option would otherwise surface only as a
    misread red gate, so the built configuration is checked here, out loud."""
    from vsdd.config import load

    config = load(Path(root) / ".factory" / "factory.toml")
    expected = {
        ("delivery", "mode"): "local",
        ("paths", "tests"): ["tests/**"],
        ("paths", "source"): ["tally/**", "tools/**"],
    }
    for (section, key), wanted in expected.items():
        got = config.get(section, key)
        if got != wanted:
            raise AssertionError(
                f"gate_project built {section}.{key} = {got!r}, expected {wanted!r}"
            )


def worktree_of(root, story: str = STORY) -> Path:
    return Path(root) / ".worktrees" / story


def commit_story(root, files: dict[str, str], message: str, story: str = STORY) -> str:
    """Write the files (paths relative to the worktree) and commit them on
    the story branch. Returns the new head."""
    tree = worktree_of(root, story)
    for relative, text in files.items():
        path = tree / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    _git(tree, "add", "-A")
    _git(tree, "commit", "-qm", message)
    return _git(tree, "rev-parse", "HEAD").strip()


def write_caller(root, agent_type: str, session_id: str = story_ready.SESSION) -> str:
    """A caller marker of one agent type; the hook runner does this for real."""
    return _write_caller(root, agent_type, session_id)


def _write_caller(root, agent_type: str, session_id: str) -> str:
    import secrets

    key = secrets.token_hex(8)
    directory = Path(root) / ".factory" / "local" / "hooks" / "calls"
    directory.mkdir(parents=True, exist_ok=True)
    (directory / f"{key}.json").write_text(
        json.dumps({
            "session_id": session_id,
            "agent_id": "a-1",
            "agent_type": agent_type,
            "cwd": str(root),
        }),
        encoding="utf-8",
    )
    return key


def env_with(root, agent_type: str) -> dict:
    key = _write_caller(root, agent_type, story_ready.SESSION)
    return {"HOME": str(Path(root).parent), "VSDD_CALLER": key}


def _git(tree: Path, *args: str) -> str:
    done = subprocess.run(["git", *args], cwd=tree, capture_output=True, text=True)
    if done.returncode != 0:
        raise AssertionError(f"git {args} failed: {done.stderr}")
    return done.stdout


def run_gate(root, *args, env) -> tuple[int, dict, str]:
    """One gate command run with --json; returns (code, result, stderr)."""
    import io

    from vsdd.cli import dispatch

    wait = [] if "--wait" in args else ["--wait", "120"]
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(
        ["-C", str(root), *args, "--json", *wait],
        stdout=out,
        stderr=err,
        stdin_tty=True,
        environ=env,
    )
    return code, (json.loads(out.getvalue()) if out.getvalue() else {}), err.getvalue()


def red_commit(root, story: str = STORY, *, source: str | None = None,
               tests: str | None = None, message: str = "the story's first commit") -> str:
    """The story's first commit: the stubbed source and the failing tests."""
    return commit_story(
        root,
        {
            "tally/blade.py": source if source is not None else BLADE_WITH_STUB,
            "tests/test_blade.py": tests if tests is not None else NEW_TESTS,
        },
        message,
        story,
    )
