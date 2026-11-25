"""Flush (plan records.md 7, test_flush): one commit, both trailers, local/
out, the product index unchanged, then clean; an invalid story file stays
out, named with its problems; a broken mount or held lock changes nothing;
and the sandbox failure comes back named, never raised."""

from __future__ import annotations

import json
import subprocess

import pytest

from .state_fixtures import make_project
from vsdd.state import commits, cycles, decisions, locks, runs
from vsdd.state.store import StoreBusy, read_record, write_record


def _status(root):
    return subprocess.run(
        ["git", "-C", str(root / ".factory"), "status", "--porcelain", "-uall"],
        capture_output=True,
        text=True,
    ).stdout


def _product_status(root):
    return subprocess.run(
        ["git", "-C", str(root), "status", "--porcelain", "-uall"],
        capture_output=True,
        text=True,
    ).stdout


_VALID_STORY = """---
id: S-1.01
title: The story
status: draft
cycle: product-1
kind: docs
priority: P1
criticality: low
contracts: []
---

## Acceptance criteria

### AC-1 (C-1)
Something observable.
"""


def test_flush(tmp_path):
    root = make_project(tmp_path)
    # One open decision sits uncommitted; one live run record sits in local/.
    cycles.cycle_write(
        root,
        {
            "id": "product-1",
            "kind": "product",
            "title": "Product cycle",
            "route": "standard",
            "opened": "2026-11-24T08:00:00Z",
            "baseline": "",
            "request": "brief",
        },
    )
    runs.run_open(root, workflow="story", cycle="product-1", session="sess-1")
    product_before = _product_status(root)

    result = commits.flush(root, "decision release", run="r-20261124-1200-ab")
    assert result["state"] == "committed"
    assert result["ok"]
    assert any(path.endswith("cycle.json") for path in result["files"])
    assert not any("/local/" in path for path in result["files"]), result["files"]

    log = subprocess.run(
        ["git", "-C", str(root / ".factory"), "log", "-1", "--format=%B"],
        capture_output=True,
        text=True,
    ).stdout
    assert log.startswith("factory: decision release")
    assert "Run: r-20261124-1200-ab" in log
    assert "Code-Head: " in log

    # Idempotent: the next flush is clean.
    assert commits.flush(root, "decision release")["state"] == "clean"

    # The product index and worktree were never touched: the product root's
    # status is what it was before the flush.
    assert _product_status(root) == product_before


def test_flush_rejects_invalid_story(tmp_path):
    root = make_project(tmp_path)
    story = root / ".factory" / "stories" / "E-1" / "S-1.01.md"
    story.parent.mkdir(parents=True, exist_ok=True)
    story.write_text("---\ntitle: Broken\ngarbage-field: yes\n---\nBody.\n", encoding="utf-8")
    note = root / ".factory" / "notes.json"
    note.write_text('{"notes": ["kept"]}', encoding="utf-8")

    result = commits.flush(root, "a broken story")
    assert result["state"] in ("partial", "committed")
    assert not result["ok"]
    rejected = {entry["path"]: entry["problems"] for entry in result["rejected"]}
    assert any(path.endswith("S-1.01.md") for path in rejected), rejected
    # The refused file stays uncommitted, named with its problems.
    assert _status(root) != ""
    committed = commits.flush(root, "the rest", run="r-1")
    files = committed["files"] if committed["state"] == "committed" else []
    assert not any(path.endswith("S-1.01.md") for path in files)


def test_flush_partial_and_busy(tmp_path, monkeypatch):
    root = make_project(tmp_path)
    # A broken mount: the .git pointer is gone, flush changes nothing.
    (root / ".factory" / ".git").rename(root / ".factory" / ".git.saved")
    result = commits.flush(root, "anything")
    assert result["state"] == "partial" and not result["ok"]
    assert result["commit"] == ""
    (root / ".factory" / ".git.saved").rename(root / ".factory" / ".git")

    # A held flush lock: the work is left to the holder. The wait is the
    # design's 5 s; the test turns it down and expects busy, not a commit.
    import fcntl
    import os

    story = root / ".factory" / "stories" / "E-1" / "S-1.01.md"
    story.parent.mkdir(parents=True, exist_ok=True)
    story.write_text(_VALID_STORY, encoding="utf-8")
    head_before = subprocess.run(
        ["git", "-C", str(root / ".factory"), "rev-parse", "HEAD"],
        capture_output=True,
        text=True,
    ).stdout
    (root / ".factory" / "local").mkdir(parents=True, exist_ok=True)
    fd = os.open(root / ".factory" / "local" / "flush.lock", os.O_RDWR | os.O_CREAT)
    fcntl.flock(fd, fcntl.LOCK_EX)
    try:
        monkeypatch.setattr(commits, "FLUSH_WAIT", 0.2)
        result = commits.flush(root, "anything")
    finally:
        fcntl.flock(fd, fcntl.LOCK_UN)
        os.close(fd)
    assert result["state"] == "busy"
    assert result["commit"] == ""
    head_after = subprocess.run(
        ["git", "-C", str(root / ".factory"), "rev-parse", "HEAD"],
        capture_output=True,
        text=True,
    ).stdout
    assert head_after == head_before, "a busy flush commits nothing"


def test_flush_never_raises(tmp_path, monkeypatch):
    """A flush reports failure with a cause; it never raises and never passes
    silently (the probe of 2026-11-24)."""
    root = make_project(tmp_path)
    from vsdd.state import gitrun

    def sandbox_refuses(cwd, *args, **kwargs):
        raise gitrun.GitError(
            "git commit exited 128: Operation not permitted", 128,
            "fatal: unable to write: Operation not permitted",
        )

    monkeypatch.setattr(gitrun, "git", sandbox_refuses)
    result = commits.flush(root, "a commit in a sandboxed Bash call")
    assert result["state"] == "failed"
    assert not result["ok"]
    assert "sandbox" in result["error"]
    assert "Operation not permitted" in result["error"]


def test_flush_surprise_is_a_failure(tmp_path, monkeypatch):
    """An unexpected error inside flush comes back as state failed, never as
    a raise (PR4C-08: flush never raises)."""
    root = make_project(tmp_path)
    story = root / ".factory" / "stories" / "E-1" / "S-1.01.md"
    story.parent.mkdir(parents=True, exist_ok=True)
    story.write_text(_VALID_STORY, encoding="utf-8")

    def explodes(cwd, *args, **kwargs):
        raise RuntimeError("the mount table vanished")

    import vsdd.state.commits as commits_module

    monkeypatch.setattr(commits_module, "_live_lock_run", explodes)
    result = commits.flush(root, "anything", run="")
    assert result["state"] == "failed"
    assert "the mount table vanished" in result["error"]


def test_flush_with_corrupt_lock_json(tmp_path):
    """A corrupt lock.json or run.json is no live run for the trailer; flush
    commits and never raises (the probe: '{broken' in lock.json)."""
    root = make_project(tmp_path)
    note = root / ".factory" / "notes.json"
    note.write_text('{"notes": ["kept"]}', encoding="utf-8")
    (root / ".factory" / "local").mkdir(parents=True, exist_ok=True)
    (root / ".factory" / "local" / "lock.json").write_text("{broken", encoding="utf-8")

    result = commits.flush(root, "a broken lock file", run="")
    assert result["state"] == "committed"
    log = subprocess.run(
        ["git", "-C", str(root / ".factory"), "log", "-1", "--format=%B"],
        capture_output=True,
        text=True,
    ).stdout
    assert "Run: -" in log

    # A corrupt run.json behind a healthy lock is the same: no live run.
    entry = {
        "locks": [
            {"scope": "proj:develop", "run": "r-20261124-1200-ab", "workflow": "story",
             "subject": "S-9.01", "session": "s", "taken": "2026-11-24T12:00:00Z",
             "seen": "2026-11-24T12:00:00Z"}
        ]
    }
    (root / ".factory" / "local" / "lock.json").write_text(json.dumps(entry), encoding="utf-8")
    (root / ".factory" / "local" / "runs" / "r-20261124-1200-ab").mkdir(parents=True)
    (root / ".factory" / "local" / "runs" / "r-20261124-1200-ab" / "run.json").write_text(
        "{broken", encoding="utf-8"
    )
    (root / ".factory" / "notes.json").write_text('{"notes": ["again"]}', encoding="utf-8")
    result = commits.flush(root, "a broken run record", run="")
    assert result["state"] == "committed"
    log = subprocess.run(
        ["git", "-C", str(root / ".factory"), "log", "-1", "--format=%B"],
        capture_output=True,
        text=True,
    ).stdout
    assert "Run: -" in log


def test_flush_commits_deletions(tmp_path):
    """A deleted record is staged and committed like any other change
    (decision D11): after flush the deletion is in HEAD and the tree is
    clean."""
    root = make_project(tmp_path)
    note = root / ".factory" / "notes.json"
    note.write_text('{"notes": ["kept"]}', encoding="utf-8")
    result = commits.flush(root, "a note to delete")
    assert result["state"] == "committed"

    note.unlink()
    result = commits.flush(root, "the note is gone")
    assert result["state"] == "committed"
    assert any(path.endswith("notes.json") for path in result["files"]), result["files"]
    gone = subprocess.run(
        ["git", "-C", str(root / ".factory"), "show", "HEAD:notes.json"],
        capture_output=True,
        text=True,
    )
    assert gone.returncode != 0, "the deleted file is not in HEAD"
    assert _status(root) == "", "the deletion is committed, the tree clean"


def test_flush_commits_deleted_decision_record_and_packet(tmp_path):
    """A committed decision record and its packet, both deleted, are staged
    and committed like any other change (decision D11): flush must not judge
    the deleted files — a capped packet would stat a missing file — and every
    later flush stays committed while the deletion stands."""
    root = make_project(tmp_path)
    decisions.decision_open(root, cycle="product-1", kind="release", packet="# Release\n")
    first = commits.flush(root, "a decision lands")
    assert first["state"] == "committed", first

    directory = root / ".factory" / "cycles" / "product-1" / "decisions"
    (directory / "release.json").unlink()
    (directory / "release.md").unlink()
    result = commits.flush(root, "the decision is withdrawn")
    assert result["state"] == "committed", result
    for name in ("release.json", "release.md"):
        shown = subprocess.run(
            ["git", "-C", str(root / ".factory"), "show", f"HEAD:cycles/product-1/decisions/{name}"],
            capture_output=True,
            text=True,
        )
        assert shown.returncode != 0, f"{name} is gone from HEAD"
    assert _status(root) == "", "both deletions committed, the tree clean"

    # A later flush, with the deletion already committed, is clean.
    assert commits.flush(root, "quiet")["state"] == "clean"


def test_flush_staged_then_deleted_new_file(tmp_path):
    """'AD': a new file staged by a killed flush and deleted from the worktree
    before the next flush neither breaks the commit nor lands in it."""
    root = make_project(tmp_path)
    story = root / ".factory" / "stories" / "E-1" / "S-1.01.md"
    story.parent.mkdir(parents=True, exist_ok=True)
    story.write_text(_VALID_STORY, encoding="utf-8")
    subprocess.run(
        ["git", "-C", str(root / ".factory"), "add", "-A", "--", "stories/E-1/S-1.01.md"],
        check=True,
        capture_output=True,
    )
    story.unlink()
    note = root / ".factory" / "notes.json"
    note.write_text('{"notes": ["kept"]}', encoding="utf-8")

    result = commits.flush(root, "recovery after a killed flush")
    assert result["state"] == "committed", result
    assert any(path.endswith("notes.json") for path in result["files"])
    assert not any(path.endswith("S-1.01.md") for path in result["files"])
    shown = subprocess.run(
        ["git", "-C", str(root / ".factory"), "show", "HEAD:stories/E-1/S-1.01.md"],
        capture_output=True,
        text=True,
    )
    assert shown.returncode != 0, "the phantom file is not in HEAD"
    assert _status(root) == ""


def test_flush_checks_only_the_changed_paths(tmp_path, monkeypatch):
    """check_paths receives exactly the changed, existing rels (PL3-3): an
    unchanged committed artifact is not re-checked on every flush."""
    root = make_project(tmp_path)
    story = root / ".factory" / "stories" / "E-1" / "S-1.01.md"
    story.parent.mkdir(parents=True, exist_ok=True)
    story.write_text(_VALID_STORY, encoding="utf-8")
    assert commits.flush(root, "the story lands")["state"] == "committed"

    note = root / ".factory" / "notes.json"
    note.write_text('{"notes": ["kept"]}', encoding="utf-8")

    from vsdd.state import artifacts

    real_check = artifacts.check_paths
    seen: dict = {}

    def spy(r, rels=None, **kwargs):
        seen["rels"] = list(rels) if rels is not None else None
        return real_check(r, rels, **kwargs)

    monkeypatch.setattr(artifacts, "check_paths", spy)
    result = commits.flush(root, "a note changes")
    monkeypatch.undo()
    assert result["state"] == "committed", result
    assert "rels" in seen, "flush never called check_paths"
    rels = seen["rels"]
    assert ".factory/notes.json" in rels, rels
    assert not any(rel.endswith("S-1.01.md") for rel in rels), rels


def test_flush_without_git_identity(tmp_path, monkeypatch):
    """Where git has no identity, flush commits as vsdd-factory (decision
    D11); the identity probe's exit 1 is the answer, not an error."""
    root = make_project(tmp_path)
    # No configured identity anywhere the probe looks.
    for key in ("user.email", "user.name"):
        subprocess.run(
            ["git", "-C", str(root / ".factory"), "config", "--unset", key],
            check=False, capture_output=True,
        )
    monkeypatch.setenv("GIT_CONFIG_GLOBAL", str(tmp_path / "empty-gitconfig"))
    monkeypatch.setenv("GIT_CONFIG_SYSTEM", str(tmp_path / "empty-gitconfig"))

    note = root / ".factory" / "notes.json"
    note.write_text('{"notes": ["kept"]}', encoding="utf-8")
    result = commits.flush(root, "anonymous")
    assert result["state"] == "committed", result
    shown = subprocess.run(
        ["git", "-C", str(root / ".factory"), "log", "-1", "--format=%an <%ae>"],
        capture_output=True,
        text=True,
    ).stdout
    assert shown.strip() == "vsdd-factory <vsdd-factory@example.invalid>"


def test_flush_leaves_a_now_rejected_stale_blob_out(tmp_path):
    """The commit is partial to the valid paths: a stale blob staged by a
    killed flush is not committed by the next flush once the file became
    rejected in the worktree."""
    root = make_project(tmp_path)
    story = root / ".factory" / "stories" / "E-1" / "S-1.01.md"
    story.parent.mkdir(parents=True, exist_ok=True)
    story.write_text(_VALID_STORY, encoding="utf-8")
    # The killed flush got as far as staging the file.
    subprocess.run(
        ["git", "-C", str(root / ".factory"), "add", "-A", "--", "stories/E-1/S-1.01.md"],
        check=True, capture_output=True,
    )
    staged = subprocess.run(
        ["git", "-C", str(root / ".factory"), "show", ":stories/E-1/S-1.01.md"],
        capture_output=True, text=True,
    ).stdout
    assert "id: S-1.01" in staged

    # Now the file is broken in the worktree: rejected, and its staged blob
    # must not ride into the commit.
    story.write_text("---\ntitle: Broken\ngarbage-field: yes\n---\nBody.\n", encoding="utf-8")
    note = root / ".factory" / "notes.json"
    note.write_text('{"notes": ["kept"]}', encoding="utf-8")

    result = commits.flush(root, "the recovery")
    assert result["state"] in ("committed", "partial")
    assert any(path.endswith("notes.json") for path in result["files"])
    assert not any(path.endswith("S-1.01.md") for path in result["files"])
    log_stories = subprocess.run(
        ["git", "-C", str(root / ".factory"), "show", "HEAD:stories/E-1/S-1.01.md"],
        capture_output=True,
        text=True,
    )
    assert log_stories.returncode != 0, "the stale staged blob stayed out of the commit"


def test_flush_commits_a_renamed_story(tmp_path):
    """A committed story renamed to a similar file commits as both its
    deletion and its addition in one flush (S3R-records-C2-1): rename
    detection in ``git diff --cached`` prints only the new name, which left
    the deletion staged and wedged every later flush."""
    root = make_project(tmp_path)
    story = root / ".factory" / "stories" / "E-1" / "S-1.01.md"
    story.parent.mkdir(parents=True, exist_ok=True)
    story.write_text(_VALID_STORY, encoding="utf-8")
    assert commits.flush(root, "the story lands")["state"] == "committed"

    moved = root / ".factory" / "stories" / "E-1" / "S-1.02.md"
    moved.write_text(_VALID_STORY.replace("S-1.01", "S-1.02"), encoding="utf-8")
    story.unlink()

    result = commits.flush(root, "the story is renumbered")
    assert result["state"] == "committed", result
    names = sorted(result["files"])
    assert "stories/E-1/S-1.01.md" in names, names
    assert "stories/E-1/S-1.02.md" in names, names
    assert _status(root) == "", "both sides of the rename are committed"
    # The next flush sees a clean tree, not a stuck staged deletion.
    assert commits.flush(root, "quiet")["state"] == "clean"


def test_flush_commits_a_deletion_already_staged(tmp_path):
    """A deletion staged before the flush — a killed flush's leftover, or a
    person's own ``git add -A`` — is committed, not refused: ``add -A``
    refuses a path that is in neither the worktree nor the index
    (S3R-records-C2-1)."""
    root = make_project(tmp_path)
    note = root / ".factory" / "notes.json"
    note.write_text('{"notes": ["kept"]}', encoding="utf-8")
    assert commits.flush(root, "a note lands")["state"] == "committed"

    note.unlink()
    subprocess.run(
        ["git", "-C", str(root / ".factory"), "add", "-A", "--", "notes.json"],
        check=True, capture_output=True,
    )
    assert _status(root).startswith("D "), _status(root)

    result = commits.flush(root, "the note is gone")
    assert result["state"] == "committed", result
    assert any(path.endswith("notes.json") for path in result["files"])
    assert _status(root) == ""


def test_flush_commits_a_rename_a_killed_flush_staged(tmp_path):
    """Both paths of a rename a killed flush staged (one ``R`` record in
    status) are committed by the next flush: the two status readers pass the
    pair over, so the index itself is what tells flush there is work
    (S3R-records-C2-1)."""
    root = make_project(tmp_path)
    story = root / ".factory" / "stories" / "E-1" / "S-1.01.md"
    story.parent.mkdir(parents=True, exist_ok=True)
    story.write_text(_VALID_STORY, encoding="utf-8")
    assert commits.flush(root, "the story lands")["state"] == "committed"

    moved = root / ".factory" / "stories" / "E-1" / "S-1.02.md"
    moved.write_text(_VALID_STORY.replace("S-1.01", "S-1.02"), encoding="utf-8")
    story.unlink()
    subprocess.run(
        ["git", "-C", str(root / ".factory"), "add", "-A", "--", "stories/E-1"],
        check=True, capture_output=True,
    )
    assert _status(root).startswith("R "), _status(root)

    result = commits.flush(root, "recovering the staged rename")
    assert result["state"] == "committed", result
    names = sorted(result["files"])
    assert "stories/E-1/S-1.01.md" in names, names
    assert "stories/E-1/S-1.02.md" in names, names
    assert _status(root) == ""


def test_flush_commits_a_renamed_name_with_space_and_accents(tmp_path):
    """A real rename of a wide name survives the whole pipe (finding
    step3-records-fix-3-C1-1): ``-z`` status and index output are never
    C-quoted, and the old and the new path are both committed."""
    root = make_project(tmp_path)
    packet = root / ".factory" / "cycles" / "product-1" / "decisions" / "nächste Runde.md"
    packet.parent.mkdir(parents=True, exist_ok=True)
    packet.write_text("# nächste Runde\n", encoding="utf-8")
    result = commits.flush(root, "a packet with a wide name")
    assert result["state"] == "committed", result
    assert any(path.endswith("nächste Runde.md") for path in result["files"])

    moved = packet.parent / "über nächste Runde.md"
    packet.rename(moved)

    result = commits.flush(root, "the packet is retitled")
    assert result["state"] == "committed", result
    names = sorted(result["files"])
    assert any(path.endswith("nächste Runde.md") for path in names), names
    assert any(path.endswith("über nächste Runde.md") for path in names), names
    shown = subprocess.run(
        ["git", "-C", str(root / ".factory"), "show",
         "HEAD:cycles/product-1/decisions/nächste Runde.md"],
        capture_output=True, text=True,
    )
    assert shown.returncode != 0, "the old name is gone from HEAD"
    shown = subprocess.run(
        ["git", "-C", str(root / ".factory"), "show",
         "HEAD:cycles/product-1/decisions/über nächste Runde.md"],
        capture_output=True, text=True,
    )
    assert shown.returncode == 0, "the new name is in HEAD"
    assert _status(root) == ""
    # The next flush sees a clean tree, not a stuck staged deletion.
    assert commits.flush(root, "quiet")["state"] == "clean"


def test_flush_does_not_commit_a_rejected_neighbour_of_a_glob_name(tmp_path):
    """State paths reach git as literal pathspecs (finding
    step3-records-fix-3-C1-1): a valid packet named ``[ab].md`` must not drag
    its rejected neighbour ``a.md`` into the commit through the pattern."""
    root = make_project(tmp_path)
    directory = root / ".factory" / "cycles" / "product-1" / "decisions"
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "[ab].md").write_text("# a valid packet\n", encoding="utf-8")
    over_cap = root / ".factory" / "cycles" / "product-1" / "decisions" / "a.md"
    over_cap.write_text("# too long\n" + "x" * 13_000, encoding="utf-8")

    result = commits.flush(root, "a glob name beside a rejected file")
    assert result["state"] == "partial", result
    assert not result["ok"]
    rejected = {entry["path"]: entry["problems"] for entry in result["rejected"]}
    assert ".factory/cycles/product-1/decisions/a.md" in rejected, rejected
    assert any(problem.startswith("over_cap") for problem in
               rejected[".factory/cycles/product-1/decisions/a.md"])

    committed = result["files"]
    assert any(path.endswith("[ab].md") for path in committed), committed
    assert not any(path.endswith("decisions/a.md") for path in committed), committed
    shown = subprocess.run(
        ["git", "-C", str(root / ".factory"), "show",
         "HEAD:cycles/product-1/decisions/[ab].md"],
        capture_output=True, text=True,
    )
    assert shown.returncode == 0, "the valid packet is committed"
    shown = subprocess.run(
        ["git", "-C", str(root / ".factory"), "show", "HEAD:cycles/product-1/decisions/a.md"],
        capture_output=True, text=True,
    )
    assert shown.returncode != 0, "the rejected file was committed through the glob"
    status = _status(root)
    assert "a.md" in status, "the rejected file stays uncommitted"


def test_flush_with_conflicting_pathspec_environment(tmp_path, monkeypatch):
    """Literal pathspecs survive a person's own pathspec environment (finding
    step3-records-fix-3-C2-1): ``GIT_LITERAL_PATHSPECS=1`` inherited beside a
    person's ``GIT_ICASE_PATHSPECS=1`` or ``GIT_GLOB_PATHSPECS=1`` makes git
    exit 128 — the settings are incompatible — so flush must force the other
    global pathspec settings off with it, and the ``[ab].md`` case keeps the
    rejected neighbour out of HEAD while it does."""
    root = make_project(tmp_path)
    monkeypatch.setenv("GIT_ICASE_PATHSPECS", "1")
    monkeypatch.setenv("GIT_GLOB_PATHSPECS", "1")
    directory = root / ".factory" / "cycles" / "product-1" / "decisions"
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "[ab].md").write_text("# a valid packet\n", encoding="utf-8")
    (directory / "a.md").write_text("# too long\n" + "x" * 13_000, encoding="utf-8")

    result = commits.flush(root, "a person's pathspec environment")
    assert result["state"] == "partial", result
    rejected = {entry["path"]: entry["problems"] for entry in result["rejected"]}
    assert ".factory/cycles/product-1/decisions/a.md" in rejected, rejected
    assert any(path.endswith("[ab].md") for path in result["files"]), result["files"]
    shown = subprocess.run(
        ["git", "-C", str(root / ".factory"), "show",
         "HEAD:cycles/product-1/decisions/[ab].md"],
        capture_output=True, text=True,
    )
    assert shown.returncode == 0, "the valid packet commits under the inherited environment"
    shown = subprocess.run(
        ["git", "-C", str(root / ".factory"), "show", "HEAD:cycles/product-1/decisions/a.md"],
        capture_output=True, text=True,
    )
    assert shown.returncode != 0, "the rejected file stays out of HEAD"
    assert "a.md" in _status(root), "the rejected file stays uncommitted"

    # A later flush under the same environment is clean, not wedged.
    (directory / "a.md").unlink()
    assert commits.flush(root, "quiet")["state"] == "clean"


def test_flush_ignores_inherited_repo_locating_environment(tmp_path, monkeypatch):
    """The repository-locating variables a person may export — ``GIT_DIR``,
    ``GIT_WORK_TREE`` and ``GIT_INDEX_FILE``; gitrun strips the object- and
    common-directory keys the same way — never redirect flush: ``cwd``
    decides the repository, so the commit still lands on the state branch and
    the foreign location receives nothing."""
    root = make_project(tmp_path)
    foreign = tmp_path / "elsewhere.git"
    subprocess.run(["git", "init", "--bare", str(foreign)], check=True, capture_output=True)
    note = root / ".factory" / "notes.json"
    note.write_text('{"notes": ["kept"]}', encoding="utf-8")
    head_before = subprocess.run(
        ["git", "-C", str(root / ".factory"), "rev-parse", "HEAD"],
        capture_output=True, text=True,
    ).stdout

    monkeypatch.setenv("GIT_DIR", str(foreign))
    monkeypatch.setenv("GIT_WORK_TREE", str(tmp_path))
    monkeypatch.setenv("GIT_INDEX_FILE", str(tmp_path / "foreign-index"))

    result = commits.flush(root, "someone exported GIT_DIR")
    # Undo before verifying: the verification's own git calls must not
    # inherit the patched environment.
    monkeypatch.undo()
    assert result["state"] == "committed", result
    head_after = subprocess.run(
        ["git", "-C", str(root / ".factory"), "rev-parse", "HEAD"],
        capture_output=True, text=True,
    ).stdout
    assert head_after != head_before, "the state branch advanced"
    shown = subprocess.run(
        ["git", "-C", str(root / ".factory"), "show", "HEAD:notes.json"],
        capture_output=True, text=True,
    )
    assert shown.returncode == 0, "the state branch got the commit"
    assert not (tmp_path / "foreign-index").exists(), "the foreign index was never used"
    refs = subprocess.run(
        ["git", "-C", str(foreign), "for-each-ref"], capture_output=True, text=True
    )
    assert refs.stdout.strip() == "", "the foreign repository received nothing"


def test_flush_refuses_an_unclaimed_record_file(tmp_path):
    """A tracked file under the record folders that no validator claims is
    refused and named (S3B-records-2): a record kind added later cannot ride
    into a commit unchecked."""
    root = make_project(tmp_path)
    stray = root / ".factory" / "cycles" / "product-1" / "extra" / "stray.json"
    stray.parent.mkdir(parents=True, exist_ok=True)
    stray.write_text('{"shape": "unknown"}', encoding="utf-8")

    result = commits.flush(root, "a stray record file")
    assert result["state"] == "partial" and not result["ok"]
    rejected = {entry["path"]: entry["problems"] for entry in result["rejected"]}
    problems = rejected[".factory/cycles/product-1/extra/stray.json"]
    assert any("no_validator" in problem for problem in problems), problems
    shown = subprocess.run(
        ["git", "-C", str(root / ".factory"), "show", "HEAD:cycles/product-1/extra/stray.json"],
        capture_output=True, text=True,
    )
    assert shown.returncode != 0, "the unclaimed file is not committed"
    # Outside the record folders a file needs no validator: it commits, while
    # the stray stays rejected and holds the flush at partial.
    note = root / ".factory" / "notes.json"
    note.write_text('{"notes": ["kept"]}', encoding="utf-8")
    again = commits.flush(root, "a record file only")
    assert any(path.endswith("notes.json") for path in again["files"]), again
    assert again["state"] == "partial" and not again["ok"]
    assert [entry["path"] for entry in again["rejected"]] == [
        ".factory/cycles/product-1/extra/stray.json"
    ]


def test_record_kinds_cover_the_shipped_tracked_records():
    """Every record schema this unit ships for a tracked record has a
    validator in flush, each row claims its own path form, and every row's
    schema is a shipped schema name (S3B-records-2)."""
    from vsdd.state.store import SCHEMA_DIR

    rows = commits.record_kinds()
    schemas = {row["schema"] for row in rows if row["schema"]}
    assert {"cycle-record", "closure-record", "decision-record"} <= schemas

    shipped = {
        path.name[: -len(".schema.json")] for path in SCHEMA_DIR.glob("*.schema.json")
    }
    assert schemas <= shipped, "a row names a schema the plugin does not ship"

    samples = {
        "cycles/product-1/cycle.json": "cycle-record",
        "cycles/product-1/units/S-9.01.json": "closure-record",
        "cycles/product-1/decisions/release.json": "decision-record",
        "cycles/product-1/decisions/release.md": "decision-packet",
        "notes.json": "notes",
    }
    for path, kind in samples.items():
        row = commits._claiming_row(path)
        assert row is not None, f"no validator claims {path}"
        assert row["kind"] == kind, path


def test_run_trailer_from_the_live_lock(tmp_path):
    root = make_project(tmp_path)
    locks.run_begin(
        root,
        scope=locks.lock_scope(root),
        workflow="story",
        subject="S-9.01",
        session="sess-A",
        cycle="product-1",
    )
    cycles.cycle_write(
        root,
        {
            "id": "product-1",
            "kind": "product",
            "title": "t",
            "route": "",
            "opened": "2026-11-24T08:00:00Z",
            "baseline": "",
            "request": "brief",
            "closed": "2026-11-24T09:00:00Z",
            "version": "0.1.0",
            "archive_commit": "abc",
        },
    )
    result = commits.flush(root, "closing", run="")
    assert result["state"] == "committed", result
    log = subprocess.run(
        ["git", "-C", str(root / ".factory"), "log", "-1", "--format=%B"],
        capture_output=True,
        text=True,
    ).stdout
    live_run = _live_run(root)
    assert f"Run: {live_run}" in log
    assert "Code-Head: " in log


def _live_run(root):
    for entry in locks.lock_entries(root):
        record = read_record(root / ".factory" / "local" / "runs" / entry["run"] / "run.json")
        if record and record.get("ended", "") == "":
            return entry["run"]
    return "-"
