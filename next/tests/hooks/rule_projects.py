"""Projects the rule tests start from (plan rules.md section 7).

``story_project`` builds on ``tests/unit/story_ready.py`` and runs the real
``vsdd survey story`` in process, so a rule sees a project with a story
worktree, a live run and recorded evidence — never hand-written record JSON.
The named fixtures of ``CASE_PROJECTS`` feed the case tables, whose cases
reference them by ``project = "<name>"``.
"""

from __future__ import annotations

import json
import secrets
from dataclasses import dataclass
from pathlib import Path

from tests.unit import story_ready
from tests.unit._repo import set_keys
from tests.unit.story_ready import make_project

STORY = "S-9.01"


@dataclass(frozen=True)
class StoryProject:
    root: Path
    story: str
    worktree: Path
    run: str
    session: str

    @property
    def head(self) -> str:
        from vsdd.hooks.gitfs import head_sha

        return head_sha(self.worktree) or ""


def story_project(tmp, kind: str = "feature", story: str = STORY, **make_kwargs) -> StoryProject:
    """A surveyed story: root, worktree at ``.worktrees/<story>``, live run;
    ``[paths]`` set so the source and tests classes exist."""
    work = Path(tmp) / f"sp-{secrets.token_hex(4)}"
    root = make_project(work, story=story, **make_kwargs)
    set_kind(root, story, kind)
    set_keys(root, "paths", source=["src/**"], tests=["tests/**", "src/**/tests.rs"])
    surveyed = survey(root, story)
    return StoryProject(
        root=root,
        story=story,
        worktree=root / ".worktrees" / story,
        run=surveyed["run_id"],
        session=story_ready.SESSION,
    )


def set_kind(root: Path, story: str, kind: str) -> None:
    """The story file's ``kind`` — before the survey, which admits the story."""
    epic = f"E-{story[2:].split('.', 1)[0]}"
    path = root / ".factory" / "stories" / epic / f"{story}.md"
    text = path.read_text(encoding="utf-8")
    lines = []
    for line in text.splitlines():
        if line.startswith("kind:"):
            lines.append(f"kind: {kind}")
        else:
            lines.append(line)
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def survey(root: Path, story: str = STORY) -> dict:
    """One in-process ``vsdd survey story``, with a caller marker."""
    key = story_ready.write_caller(root)
    environ = {"HOME": str(Path(root).parent), "VSDD_CALLER": key}
    code, out, err = story_ready.run_vsdd(
        root, "survey", "story", "--subject", story, "--json", environ=environ
    )
    data = json.loads(out) if out.strip() else {}
    if code != 0 or not data.get("ok"):
        raise AssertionError(f"survey story failed ({code}): {err}{out}")
    return data


# --- evidence ---------------------------------------------------------------


def _sha_of(root: Path, worktree: Path) -> str:
    from vsdd.hooks.gitfs import head_sha

    return head_sha(worktree) or head_sha(root) or "0" * 12


def evidence(p: StoryProject, gate: str, passed: bool, **extra) -> dict:
    """One evidence record for the project's story at its head, written
    through the store's ``evidence_write``."""
    from vsdd.state.gate_evidence import evidence_write

    record = {
        "gate": gate,
        "subject": p.story,
        "sha": _sha_of(p.root, p.worktree),
        "passed": passed,
    }
    record.update(extra)
    return evidence_write(p.root, record)


def red(p: StoryProject, frozen: list[str] | None = None, refreeze: bool = False) -> dict:
    """A passing red record with a frozen set (default: one frozen test)."""
    names = frozen if frozen is not None else ["tests/test_sample.py"]
    return evidence(
        p,
        "red",
        True,
        refreeze=refreeze,
        frozen=[{"file": name, "sha": "aa"} for name in names],
        tests=[f"tests::{name}" for name in names],
        criteria={"AC-1": [f"tests::{name}" for name in names]},
    )


def green(p: StoryProject) -> dict:
    return evidence(p, "green", True)


def commit(p: StoryProject, name: str = "src/committed.rs") -> str:
    """One real commit in the story worktree; the head after it. The evidence
    lookups are not tied to the head, so records written at an earlier commit
    keep holding across it (rules/tests-first.md)."""
    import os
    import subprocess

    path = p.worktree / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("// committed\n", encoding="utf-8")
    env = dict(os.environ)
    env.update({
        "GIT_AUTHOR_NAME": "t", "GIT_AUTHOR_EMAIL": "t@t",
        "GIT_COMMITTER_NAME": "t", "GIT_COMMITTER_EMAIL": "t@t",
    })
    for args in (["add", "-A"], ["commit", "-qm", "advance the story branch"]):
        subprocess.run(["git", "-C", str(p.worktree), *args], check=True,
                       capture_output=True, env=env)
    return p.head


def freeze(root: Path, names: list[str]) -> None:
    """Write the frozen test files into the story worktree, so a frozen path
    exists to protect."""
    worktree = root / ".worktrees" / STORY
    for name in names:
        path = worktree / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("def test_sample():\n    assert True\n", encoding="utf-8")


def step_result(p: StoryProject, **changes) -> dict:
    """A minimal stored step result for the project's run, with overrides
    applied key by key (``step=``. ``data=``. whole-dict replacements)."""
    result = {
        "step": {
            "run": p.run,
            "workflow": "story",
            "subject": p.story,
            "name": "implement",
            "attempt": 1,
            "method": "tdd",
        },
        "status": "done",
        "summary": "the step went",
        "head": _sha_of(p.root, p.worktree),
        "data": {},
    }
    for key, value in changes.items():
        if isinstance(value, dict) and isinstance(result.get(key), dict):
            result[key].update(value)
        else:
            result[key] = value
    return result


# --- the named fixtures of the case tables ----------------------------------


def _story_fixture(tmp_path, kind: str = "feature"):
    p = story_project(tmp_path, kind=kind)
    if kind in ("feature", "fix", "port"):
        freeze(p.root, ["tests/test_sample.py"])
    return p


def develop_checkout(tmp_path):
    """A project whose main checkout sits on the integration branch."""
    root = make_project(tmp_path)
    return root, str(root), {"root": str(root)}


def story_before_red(tmp_path):
    p = _story_fixture(tmp_path)
    return p.root, str(p.worktree), _values(p)


def story_after_red(tmp_path):
    p = _story_fixture(tmp_path)
    red(p)
    return p.root, str(p.worktree), _values(p)


def story_committed(tmp_path):
    """Red, then one implementer commit on the story branch: the record
    holds at the later commit (rules/tests-first.md)."""
    p = _story_fixture(tmp_path)
    red(p)
    commit(p)
    return p.root, str(p.worktree), _values(p)


def live_run(tmp_path):
    """A surveyed story with a passing green record at its head."""
    p = _story_fixture(tmp_path)
    red(p)
    green(p)
    return p.root, str(p.worktree), _values(p)


def _values(p: StoryProject) -> dict:
    return {
        "root": str(p.root),
        "worktree": str(p.worktree),
        "story": p.story,
        "run": p.run,
        "sha": _sha_of(p.root, p.worktree),
    }


CASE_PROJECTS = {
    "develop-checkout": develop_checkout,
    "story-before-red": story_before_red,
    "story-after-red": story_after_red,
    "story-committed": story_committed,
    "live-run": live_run,
}


def case_project(name: str, tmp_path):
    """One named fixture: (root, cwd, values)."""
    return CASE_PROJECTS[name](tmp_path)
