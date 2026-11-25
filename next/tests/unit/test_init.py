"""``vsdd init``: create, mount, legacy, partial, idempotence and detection
(state-core.md 3.6, decision 3; tested on make_sample_repo.sh -n copies)."""

from __future__ import annotations

import io
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

from vsdd.cli import dispatch

from ._repo import make_project, run_init

SAMPLE_SCRIPT = Path(__file__).resolve().parents[2] / "tests" / "e2e" / "make_sample_repo.sh"
BIN_VSDD = Path(__file__).resolve().parents[2] / "plugins" / "vsdd-factory" / "bin" / "vsdd"


def sample_copy(tmp_path: Path) -> Path:
    dest = tmp_path / "proj"
    proc = subprocess.run(
        ["sh", str(SAMPLE_SCRIPT), "-n", str(dest)], capture_output=True, text=True
    )
    assert proc.returncode == 0, proc.stderr
    return dest


def init(root: Path, *args: str, env=None) -> tuple[int, dict, str]:
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(
        ["-C", str(root), "init", *args],
        stdout=out,
        stderr=err,
        stdin_tty=True,
        environ=env if env is not None else {"HOME": str(root.parent)},
    )
    payload = json.loads(out.getvalue()) if out.getvalue().strip() else {}
    return code, payload, err.getvalue()


def test_init_creates_and_mounts(tmp_path):
    root = sample_copy(tmp_path)
    code, data, err = init(root)
    assert (code, data["state"]) == (0, "ready"), err
    assert data["created"] is True and data["mounted"] is True
    state = root / ".factory"
    assert (state / "factory.toml").is_file()
    assert (state / "notes.json").is_file()
    subprocess.run(["git", "status"], cwd=root, check=True, capture_output=True)
    # The state worktree is mounted on the state branch.
    head = (state / ".git").read_text(encoding="utf-8")
    assert "gitdir:" in head
    # The exclusions exist.
    exclude = (root / ".git" / "info" / "exclude").read_text(encoding="utf-8")
    assert ".factory/" in exclude and ".worktrees/" in exclude
    # The person's HEAD, index and tree are untouched.
    product = subprocess.run(
        ["git", "status", "--porcelain"], cwd=root, capture_output=True, text=True
    ).stdout
    assert product.strip() == ""


def test_init_is_idempotent(tmp_path):
    root = sample_copy(tmp_path)
    code, first, _ = init(root)
    assert code == 0
    before = (root / ".factory" / "factory.toml").read_text(encoding="utf-8")
    code, second, _ = init(root)
    assert (code, second["state"]) == (0, "ready")
    assert second["created"] is False and second["mounted"] is False
    assert (root / ".factory" / "factory.toml").read_text(encoding="utf-8") == before


def test_init_mounts_existing_branch(tmp_path):
    root = sample_copy(tmp_path)
    code, _, err = init(root)
    assert code == 0, err
    subprocess.run(
        ["git", "worktree", "remove", str(root / ".factory"), "--force"],
        cwd=root, check=True, capture_output=True,
    )
    # Local branch exists, no mount: init mounts it.
    code, data, err = init(root)
    assert (code, data["state"]) == (0, "ready"), err
    assert data["created"] is False and data["mounted"] is True


def test_init_resumes_after_interruption(tmp_path, monkeypatch):
    """Killed between the branch creation and the mount: the next run mounts
    and finishes (finding S3R-state-core-14, the after-the-branch case)."""
    from vsdd.state import cmd_init

    root = sample_copy(tmp_path)
    # Die right after the branch exists, before any mount.
    def killed(root):
        raise RuntimeError("killed after the branch")

    monkeypatch.setattr(cmd_init, "mount_state", killed)
    code, data, err = init(root)
    assert code == 2
    assert "killed after the branch" in err
    assert "factory-artifacts" in subprocess.run(
        ["git", "for-each-ref", "refs/heads"], cwd=root, capture_output=True, text=True
    ).stdout
    # The re-run mounts what the dead run created.
    monkeypatch.undo()
    code, data, err = init(root)
    assert (code, data["state"]) == (0, "ready"), err
    assert data["created"] is False and data["mounted"] is True


def test_init_resumes_after_a_killed_exclusion_step(tmp_path, monkeypatch):
    """Killed between the mount and the exclusions: the next run, finding a
    ready project, still writes them (finding S3R-state-core-2a)."""
    from vsdd.state import cmd_init

    root = sample_copy(tmp_path)
    real = cmd_init.ensure_excludes

    def killed(root):
        return None  # the step that dies before writing anything

    monkeypatch.setattr(cmd_init, "ensure_excludes", killed)
    code, data, err = init(root)
    assert (code, data["state"]) == (0, "ready"), err
    monkeypatch.undo()
    exclude = root / ".git" / "info" / "exclude"
    assert ".factory/" not in exclude.read_text(encoding="utf-8")
    # Without the fix, this second run reports ready and never writes them.
    code, data, err = init(root)
    assert (code, data["state"]) == (0, "ready"), err
    assert ".factory/" in exclude.read_text(encoding="utf-8")
    product = subprocess.run(
        ["git", "status", "--porcelain"], cwd=root, capture_output=True, text=True
    ).stdout
    assert "?? .factory/" not in product


def test_init_finishes_a_worktree_add_killed_halfway(tmp_path):
    """Killed inside 'git worktree add': a registered worktree whose
    directory moved aside no longer wedges the next run (S3R-state-core-2b)."""
    root = sample_copy(tmp_path)
    code, _, err = init(root)
    assert code == 0, err
    # The person moves the half-built state directory aside.
    subprocess.run(["rm", "-rf", str(root / ".factory")], check=True)
    code, data, err = init(root)
    assert (code, data["state"]) == (0, "ready"), err
    assert (root / ".factory" / "factory.toml").is_file()


def test_init_accepts_an_interrupted_mount_pointer(tmp_path):
    """A .factory holding only the worktree's own .git pointer is an
    interrupted mount, not junk to move aside (finding S3R-state-core-2b)."""
    root = sample_copy(tmp_path)
    code, _, err = init(root)
    assert code == 0, err
    pointer = (root / ".factory" / ".git").read_text(encoding="utf-8")
    subprocess.run(["rm", "-rf", str(root / ".factory")], check=True)
    (root / ".factory").mkdir()
    (root / ".factory" / ".git").write_text(pointer, encoding="utf-8")
    code, data, err = init(root)
    assert (code, data["state"]) == (0, "ready"), err
    assert (root / ".factory" / "factory.toml").is_file()


def test_init_origin_mount(tmp_path):
    """A fresh clone mounts the state branch from origin (plan: 'local and
    origin'); the local branch exists and tracks it (S3R-state-core-14)."""
    root = sample_copy(tmp_path)
    code, _, err = init(root)
    assert code == 0, err
    bare = tmp_path / "remote.git"
    subprocess.run(
        ["git", "init", "-q", "--bare", str(bare)], check=True, capture_output=True
    )
    subprocess.run(["git", "push", "-q", str(bare), "main", "factory-artifacts"], cwd=root, check=True)
    clone = tmp_path / "clone"
    subprocess.run(["git", "clone", "-q", str(bare), str(clone)], check=True, capture_output=True)
    # A clone has no identity of its own: set one, so the test holds wherever
    # no global identity exists (finding S3R-state-core-C1-3).
    for key, value in (("user.email", "factory@example.test"), ("user.name", "Factory Test")):
        subprocess.run(["git", "config", key, value], cwd=clone, check=True)
    code, data, err = init(clone)
    assert (code, data["state"]) == (0, "ready"), err
    assert data["created"] is False and data["mounted"] is True
    # The upstream branch: the local name exists and tracks origin's.
    upstream = subprocess.run(
        ["git", "rev-parse", "--abbrev-ref", "factory-artifacts@{upstream}"],
        cwd=clone, capture_output=True, text=True, check=True,
    ).stdout.strip()
    assert upstream == "origin/factory-artifacts"


def test_init_concurrent_creation_keeps_the_first_branch(tmp_path):
    """A second init losing the create race mounts the winner instead of
    silently replacing the state branch (finding S3R-state-core-3)."""
    from vsdd.state.cmd_init import _create_branch
    from vsdd.state.detection import render_config

    root = sample_copy(tmp_path)
    text, _, _ = render_config(root)
    assert _create_branch(root, text) is True
    first = subprocess.run(
        ["git", "rev-parse", "refs/heads/factory-artifacts"],
        cwd=root, check=True, capture_output=True, text=True,
    ).stdout.strip()
    code, data, err = init(root)
    assert (code, data["state"]) == (0, "ready"), err
    assert data["created"] is False
    second = subprocess.run(
        ["git", "rev-parse", "refs/heads/factory-artifacts"],
        cwd=root, check=True, capture_output=True, text=True,
    ).stdout.strip()
    assert second == first  # not replaced


def test_create_branch_twice_keeps_the_first_commit(tmp_path):
    """The second _create_branch loses the update-ref (an empty old value) and
    reports False with the tip unchanged: the loser of the race never replaces
    the winner's commit (finding S3R-state-core-3, exercised directly)."""
    from vsdd.state.cmd_init import _create_branch
    from vsdd.state.detection import render_config

    root = sample_copy(tmp_path)
    text, _, _ = render_config(root)
    assert _create_branch(root, text) is True
    first = subprocess.run(
        ["git", "rev-parse", "refs/heads/factory-artifacts"],
        cwd=root, check=True, capture_output=True, text=True,
    ).stdout.strip()
    assert _create_branch(root, text) is False
    second = subprocess.run(
        ["git", "rev-parse", "refs/heads/factory-artifacts"],
        cwd=root, check=True, capture_output=True, text=True,
    ).stdout.strip()
    assert second == first


def test_init_losing_a_concurrent_create_mounts_the_winner(tmp_path, monkeypatch):
    """Init whose own update-ref raced away takes the losing path: it mounts
    the winner's branch and reports created=False (finding
    S3R-state-core-3, the path inside init)."""
    from vsdd.state import cmd_init

    root = sample_copy(tmp_path)
    real_create = cmd_init._create_branch

    def losing_create(root, text):
        real_create(root, text)  # the branch lands as the winner wrote it
        return False  # this run's update-ref lost the race

    monkeypatch.setattr(cmd_init, "_create_branch", losing_create)
    code, data, err = init(root)
    assert (code, data["state"]) == (0, "ready"), err
    assert data["created"] is False and data["mounted"] is True
    assert (root / ".factory" / "factory.toml").is_file()


def test_init_refuses_a_non_empty_state_directory_without_leaving_a_branch(tmp_path):
    """Neither `none` nor `unmounted` creates the branch before refusing a
    non-empty .factory (finding S3R-state-core-4)."""
    for planted in ("junk.txt", "nested/junk.txt"):
        root = sample_copy(tmp_path / planted.replace("/", "_"))
        state = root / ".factory"
        (state / "nested").mkdir(parents=True)
        (state / "junk.txt").write_text("not state\n", encoding="utf-8")
        code, data, err = init(root)
        assert code == 2, err
        assert "move it aside" in err
        branches = subprocess.run(
            ["git", "for-each-ref", "refs/heads"], cwd=root, capture_output=True, text=True
        ).stdout
        assert "factory-artifacts" not in branches, planted


def test_init_refuses_a_plain_file_state_directory_without_leaving_a_branch(tmp_path):
    """A plain file at .factory is refused before anything is created: the
    old order died in `worktree add` with the branch already made
    (finding S3R-state-core-C1-4, the S3R-4 failure)."""
    root = sample_copy(tmp_path)
    (root / ".factory").write_text("not a directory\n", encoding="utf-8")
    code, data, err = init(root)
    assert code == 2, err
    assert "move it aside" in err
    branches = subprocess.run(
        ["git", "for-each-ref", "refs/heads"], cwd=root, capture_output=True, text=True
    ).stdout
    assert "factory-artifacts" not in branches
    assert (root / ".factory").is_file()  # the person's file is untouched


def test_init_refuses_a_nested_repository_and_leaves_it_whole(tmp_path):
    """A .factory that is a repository of its own (its one file deleted, only
    .git left) is data, not an interrupted mount: init refuses and its history
    survives (finding S3R-state-core-C1-1)."""
    root = sample_copy(tmp_path)
    nested = root / ".factory"
    nested.mkdir()
    subprocess.run(["git", "init", "-q", str(nested)], check=True, capture_output=True)
    subprocess.run(
        ["git", "config", "user.email", "factory@example.test"], cwd=nested, check=True
    )
    subprocess.run(["git", "config", "user.name", "Factory Test"], cwd=nested, check=True)
    (nested / "precious.md").write_text("# history\n", encoding="utf-8")
    subprocess.run(["git", "add", "-A"], cwd=nested, check=True)
    subprocess.run(["git", "commit", "-qm", "precious"], cwd=nested, check=True)
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=nested, check=True, capture_output=True, text=True
    ).stdout.strip()
    # The repro: the worktree's one file is deleted, so only .git remains.
    (nested / "precious.md").unlink()
    code, data, err = init(root)
    assert code == 2, err
    assert "move it aside" in err
    # The nested repository survived with its history.
    assert (nested / ".git").is_dir()
    assert head == subprocess.run(
        ["git", "rev-parse", "HEAD"], cwd=nested, check=True, capture_output=True, text=True
    ).stdout.strip()
    branches = subprocess.run(
        ["git", "for-each-ref", "refs/heads"], cwd=root, capture_output=True, text=True
    ).stdout
    assert "factory-artifacts" not in branches


def test_init_reports_an_unmounted_legacy_branch_without_mounting(tmp_path):
    """A legacy branch (STATE.md on factory-artifacts) is reported: nothing is
    mounted and next holds the import commands (finding S3R-state-core-5)."""
    root = sample_copy(tmp_path)
    # Build a state branch holding only STATE.md, as the 0.x importer would.
    subprocess.run(
        ["git", "checkout", "-q", "--orphan", "factory-artifacts"], cwd=root, check=True
    )
    (root / "STATE.md").write_text("# pre-1.0 state\n", encoding="utf-8")
    subprocess.run(["git", "add", "STATE.md"], cwd=root, check=True)
    subprocess.run(
        ["git", "commit", "-qm", "legacy"], cwd=root, check=True, capture_output=True
    )
    subprocess.run(["git", "checkout", "-q", "main"], cwd=root, check=True)
    code, data, err = init(root)
    assert (code, data["state"]) == (1, "legacy"), err
    assert data["mounted"] is False
    assert not (root / ".factory").exists()
    assert any("import" in step for step in data["next"])
    assert any("dry-run" in step for step in data["next"])


def test_init_auto_allow_default_off(tmp_path):
    root = sample_copy(tmp_path)
    code, data, err = init(root)
    assert code == 0, err
    cfg = config_of(root)
    scope = cfg["hooks"]["rules"].get("auto-allow", {}).get("scope")
    assert scope == "off"


def config_of(root: Path) -> dict:
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(
        ["-C", str(root), "config", "--json"],
        stdout=out,
        stderr=err,
        stdin_tty=True,
        environ={"HOME": str(root.parent)},
    )
    assert code == 0, err.getvalue()
    return json.loads(out.getvalue())["config"]


def test_init_legacy_converts_nothing(tmp_path):
    root = sample_copy(tmp_path)
    state = root / ".factory"
    state.mkdir()
    (state / "STATE.md").write_text("# pre-1.0 state\n", encoding="utf-8")
    code, data, err = init(root)
    assert (code, data["state"]) == (1, "legacy"), err
    assert data["created"] is False and data["mounted"] is False
    assert any("import" in step for step in data["next"])
    assert (state / "STATE.md").is_file()
    assert not (state / "factory.toml").exists()


def test_init_partial_refuses(tmp_path):
    # SL1-27 + plan 3.6: a plain .factory/ holding a factory.toml is
    # `partial`; init exits 1 with the repair named in next.
    root = sample_copy(tmp_path)
    state = root / ".factory"
    state.mkdir()
    (state / "factory.toml").write_text("schema = 1\n", encoding="utf-8")
    code, data, err = init(root)
    assert code == 1
    assert data["state"] == "partial"
    assert any("/vsdd-factory:status --fix" in step for step in data["next"])
    assert data["mounted"] is False
    assert not (root / ".factory" / "factory.toml").exists() or (root / ".factory" / "factory.toml").read_text(encoding="utf-8") == "schema = 1\n"


def test_init_not_a_repository_exits_2(tmp_path):
    plain = tmp_path / "plain"
    plain.mkdir()
    code, data, err = init(plain)
    assert code == 2 and "not a git repository" in err


def test_init_detection(tmp_path):
    root = sample_copy(tmp_path)
    code, data, err = init(root)
    assert code == 0, err
    detected = data["detected"]
    assert detected["project.name"]
    assert detected["delivery.mode"] == "local"  # the copy has no remote
    assert "README.md" in detected["paths.docs"]
    assert "CLAUDE.md" in detected["paths.docs"]  # finding VSA-17
    cfg = config_of(root)
    assert "CLAUDE.md" in cfg["paths"]["docs"]
    assert cfg["delivery"]["mode"] == "local"
    # The detected release branch is the protected one (S3R-state-core-23).
    assert cfg["branches"]["release"] == detected["branches.release"]
    assert cfg["branches"]["protected"] == [cfg["branches"]["release"]]
    # Commands stay empty and are listed in undetected.
    assert cfg["commands"]["test"] == ""
    assert data["undetected"] and all(u.startswith("commands.") for u in data["undetected"])


def test_init_detection_protects_a_master_only_repository(tmp_path):
    """A repository releasing from master is protected at master: the SPECS
    default ['main'] equals the sample's detected release branch, so only a
    non-main release can fail without the derivation (finding
    S3R-state-core-23)."""
    root = tmp_path / "masterly"
    root.mkdir()
    subprocess.run(["git", "init", "-q", "-b", "master", str(root)], check=True)
    subprocess.run(
        ["git", "config", "user.email", "factory@example.test"], cwd=root, check=True
    )
    subprocess.run(["git", "config", "user.name", "Factory Test"], cwd=root, check=True)
    (root / "README.md").write_text("# masterly\n", encoding="utf-8")
    subprocess.run(["git", "add", "-A"], cwd=root, check=True)
    subprocess.run(["git", "commit", "-qm", "x"], cwd=root, check=True)
    code, data, err = init(root)
    assert (code, data["state"]) == (0, "ready"), err
    assert data["detected"]["branches.release"] == "master"
    assert data["detected"]["branches.protected"] == ["master"]
    cfg = config_of(root)
    assert cfg["branches"]["release"] == "master"
    assert cfg["branches"]["protected"] == ["master"]


def test_init_through_bin_vsdd(tmp_path):
    root = sample_copy(tmp_path)
    # uv's and the interpreter's directories ride on the PATH: the test must
    # not depend on where uv is installed or on a .venv in the working
    # directory, and uv must find a Python 3.11+ even in the Linux
    # ~/.local/bin layout, where it lives apart from every interpreter
    # (S3R-state-core-28).
    path_parts = ["/usr/local/bin", "/usr/bin", "/bin"]
    for tool in (shutil.which("uv"), sys.executable):
        if tool:
            parent = str(Path(tool).parent)
            if parent not in path_parts:
                path_parts.insert(0, parent)
    proc = subprocess.run(
        [str(BIN_VSDD), "-C", str(root), "init", "--json"],
        capture_output=True,
        text=True,
        env={"PATH": ":".join(path_parts), "HOME": str(root)},
    )
    assert proc.returncode == 0, proc.stderr
    payload = json.loads(proc.stdout)
    assert (payload["ok"], payload["state"]) == (True, "ready")
