"""Path classes (plan hook-runner-b 3.1, 7; test_classes.py)."""

from __future__ import annotations

import shutil
import tempfile
from pathlib import Path

from tests.hooks import runner_helpers as helpers
from tests.hooks.test_runner import base_env, read_events
from vsdd.hooks.model import Advise, Ctx, RuleEntry

READER = "tests.hooks.test_classes"


def check(ctx, cfg):
    """The fixture rule of test_invalid_policy_is_the_rules_fault: it reads
    ``ctx.classes`` and says so. Imported by RuleEntry.load through READER."""
    return Advise("classes_probe", ",".join(ctx.classes))


def make_ctx(project, overrides=None, env_extra=None, tool="Edit"):
    payload = helpers.payload("PreToolUse/edit-main.json")
    payload["cwd"] = str(project)
    payload["tool_name"] = tool
    payload.update(overrides or {})
    return Ctx(payload, base_env(project, env_extra), "gate", str(project))


def edit(project, relative):
    return {"tool_input": {"file_path": str(project / relative)}}


def test_state_classes_innermost_first(tmp_path):
    project = helpers.make_project(tmp_path)
    (project / ".factory" / "specs" / "product").mkdir(parents=True)
    (project / ".factory" / "specs" / "product" / "brief.md").write_text("b\n")
    (project / ".factory" / "local" / "runs").mkdir(parents=True)
    (project / ".factory" / "local" / "runs" / "r.json").write_text("{}\n")

    brief = make_ctx(project, edit(project, ".factory/specs/product/brief.md"))
    # The contained class before its container; a state path is never "project".
    assert brief.classes == ("brief", "spec-product")

    run = make_ctx(project, edit(project, ".factory/local/runs/r.json"))
    assert run.classes == ("runs", "local")

    config = make_ctx(project, edit(project, ".factory/factory.toml"))
    assert config.classes == ("config",)

    # The same answer from the function itself, target or not:
    assert brief.classes_of(project / ".factory" / "specs" / "product" / "brief.md") == (
        "brief",
        "spec-product",
    )


def test_product_class_by_precedence(tmp_path):
    project = helpers.make_project(tmp_path)
    (project / ".factory" / "factory.toml").write_text(
        'schema = 1\n[paths]\ntests = ["src/**/tests.rs"]\nsource = ["src/**"]\n',
        encoding="utf-8",
    )
    (project / "src" / "parser").mkdir(parents=True)

    main = make_ctx(project, edit(project, "src/parser/tests.rs"))
    assert main.classes == ("tests", "project")

    # A product path in a story worktree is classed against the worktree root:
    worktree = project / ".worktrees" / "S-1.02"
    worktree.mkdir(parents=True)
    git_dir = project / ".git" / "worktrees" / "S-1.02"
    git_dir.mkdir(parents=True)
    (worktree / ".git").write_text(f"gitdir: {git_dir}\n", encoding="utf-8")
    (git_dir / "HEAD").write_text("ref: story/S-1.02\n", encoding="utf-8")
    inside = make_ctx(project, {
        "cwd": str(worktree),
        "tool_input": {"file_path": str(worktree / "src" / "parser" / "tests.rs")},
    })
    # cwd is inside the worktree, so it is also the caller's own:
    assert inside.classes == ("tests", "project", "own-worktree")
    # ... and, `where` naming the worktree, it is also the caller's own:
    assert inside.classes_of(worktree / "src" / "parser" / "tests.rs", where=str(worktree)) == (
        "tests",
        "project",
        "own-worktree",
    )

    # A path no glob matches is unclassified (still project):
    stray = make_ctx(project, edit(project, "readme.md"))
    assert stray.classes == ("unclassified", "project")


def test_computed_classes(tmp_path, monkeypatch):
    fake_temp = tmp_path / "temproot"
    fake_temp.mkdir()
    monkeypatch.setattr(tempfile, "tempdir", str(fake_temp))

    project = helpers.make_project(tmp_path)
    worktree = project / ".worktrees" / "S-1.02"
    worktree.mkdir(parents=True)
    git_dir = project / ".git" / "worktrees" / "S-1.02"
    git_dir.mkdir(parents=True)
    (worktree / ".git").write_text(f"gitdir: {git_dir}\n", encoding="utf-8")
    (git_dir / "HEAD").write_text("ref: story/S-1.02\n", encoding="utf-8")

    # project: inside the project, outside the state directory. tmp: the
    # system temporary directory. outside: under none of them.
    inside = make_ctx(project, edit(project, "a.rs"))
    assert inside.classes == ("unclassified", "project")
    assert make_ctx(project, {"tool_input": {"file_path": str(fake_temp / "f.rs")}}).classes == (
        "unclassified",
        "tmp",
    )
    assert make_ctx(project, {"tool_input": {"file_path": "/opt/vsdd-classes-probe/f.rs"}}).classes == (
        "unclassified",
        "outside",
    )

    # The plugin directory is neither project nor outside, when the platform
    # named it:
    plugin = tmp_path / "plugin"
    plugin_ctx = make_ctx(
        project,
        {"tool_input": {"file_path": str(plugin / "py" / "x.py")}},
        env_extra={"CLAUDE_PLUGIN_ROOT": str(plugin)},
    )
    assert plugin_ctx.classes == ("unclassified",)

    # own-worktree, by `where`, which defaults to the payload cwd:
    own = make_ctx(project, {
        "cwd": str(worktree),
        "tool_input": {"file_path": str(worktree / "n.md")},
    })
    assert "own-worktree" in own.classes
    # A path in no worktree is never own-worktree:
    assert "own-worktree" not in inside.classes
    # ... and `where` given by hand decides for a path of that worktree:
    assert "own-worktree" in make_ctx(project, edit(project, "a.rs")).classes_of(
        worktree / "n.md", where=str(worktree)
    )


def test_real_temporary_directory_is_tmp(tmp_path):
    """A real temporary file is ``tmp``: the system temporary directory is
    compared resolved, so macOS's ``/var`` symlink does not turn ``tmp``
    into ``outside`` (S3R-hook-runner-b-3)."""
    project = helpers.make_project(tmp_path)
    real_dir = Path(tempfile.mkdtemp(prefix="vsdd-classes-tmp-"))
    try:
        (real_dir / "f.rs").write_text("x\n", encoding="utf-8")
        ctx = make_ctx(project, {"tool_input": {"file_path": str(real_dir / "f.rs")}})
        assert ctx.classes == ("unclassified", "tmp")
    finally:
        shutil.rmtree(real_dir, ignore_errors=True)


def test_project_inside_temporary_directory_is_not_tmp(tmp_path):
    """A project may sit under the system temporary directory (Linux CI, a
    ramdisk); its paths are project and state paths, never also ``tmp`` —
    the regions of classes.toml's closing comment are separate
    (S3R-hook-runner-b-4)."""
    outer = Path(tempfile.mkdtemp(prefix="vsdd-classes-proj-"))
    try:
        project = helpers.make_project(outer / "deep")
        (project / "a.rs").write_text("x\n", encoding="utf-8")
        product = make_ctx(project, edit(project, "a.rs"))
        assert product.classes == ("unclassified", "project")
        state = make_ctx(project, edit(project, ".factory/factory.toml"))
        assert state.classes == ("config",)
    finally:
        shutil.rmtree(outer, ignore_errors=True)


def test_symlinked_target_classes_both_forms(tmp_path):
    """A write through a symlink is judged by the destination and by the
    place the call names (rule-model 2.5: class matching tests both the
    literal and the resolved path; S3R-hook-runner-b-6)."""
    project = helpers.make_project(tmp_path)
    (project / ".factory" / "specs" / "product").mkdir(parents=True, exist_ok=True)
    (project / "src").mkdir()
    (project / "src" / "main.rs").write_text("x\n", encoding="utf-8")
    (project / ".factory" / "factory.toml").write_text(
        "schema = 1\n[paths]\nsource = [\"src/**\"]\n", encoding="utf-8"
    )
    link = project / ".factory" / "specs" / "product" / "brief.md"
    link.symlink_to(project / "src" / "main.rs")

    ctx = make_ctx(project, edit(project, ".factory/specs/product/brief.md"))
    # The literal form's state classes, innermost first, then the resolved
    # form's product class and project:
    assert ctx.classes == ("brief", "spec-product", "source", "project")
    # The same answer from the function given the literal path — the call
    # names the place, the symlink names the destination, and both count:
    assert ctx.classes_of(ctx.target_literal) == ctx.classes


def test_spelling_through_symlinked_alias_classes_both_forms(tmp_path):
    """A ``file_path`` spelled through a symlinked ancestor of the project
    still classes both forms: the spelling matches no spelling of the resolved
    root, so the literal is anchored at the resolved parent with the leaf kept
    literal (S3R-hook-runner-b-6). The macOS ``/var`` case, spelled
    portably."""
    project = helpers.make_project(tmp_path)
    (project / ".factory" / "specs" / "product").mkdir(parents=True, exist_ok=True)
    (project / "src").mkdir()
    (project / "src" / "main.rs").write_text("x\n", encoding="utf-8")
    (project / ".factory" / "factory.toml").write_text(
        "schema = 1\n[paths]\nsource = [\"src/**\"]\n", encoding="utf-8"
    )
    link = project / ".factory" / "specs" / "product" / "brief.md"
    link.symlink_to(project / "src" / "main.rs")

    # The call spells the brief through an alias of tmp_path, an ancestor of
    # the root, so the literal is under no spelling of the resolved project
    # root — the case macOS ships for every /var-prefixed project:
    alias = tmp_path / "alias"
    alias.symlink_to(tmp_path, target_is_directory=True)
    spelled = alias / "factory-project" / ".factory" / "specs" / "product" / "brief.md"
    ctx = make_ctx(project, {"tool_input": {"file_path": str(spelled)}})
    assert ctx.classes == ("brief", "spec-product", "source", "project")


def test_in_project_symlink_outside_stays_outside(tmp_path):
    """An in-project symlink whose destination lies outside the project, the
    plugin directory and tmp keeps ``outside``: the union keeps every class of
    both forms (S3R-hook-runner-b-C1-1) — a wall admitting ``project`` must
    not wave /etc/hosts through the link."""
    project = helpers.make_project(tmp_path)
    (project / "src").mkdir()
    (project / "src" / "wall-peek").symlink_to("/etc/hosts")

    ctx = make_ctx(project, edit(project, "src/wall-peek"))
    assert ctx.classes == ("unclassified", "project", "outside")


def test_invalid_policy_is_the_rules_fault(tmp_path, monkeypatch):
    import vsdd.hooks.policy as hooks_policy
    import vsdd.policy as vsdd_policy

    project = helpers.make_project(tmp_path)
    (project / ".factory" / "specs" / "product").mkdir(parents=True)
    (project / ".factory" / "specs" / "product" / "brief.md").write_text("b\n")
    (project / "src").mkdir()
    (project / "src" / "main.rs").write_text("x\n", encoding="utf-8")

    original_load = hooks_policy._load_table

    def broken(name):
        raise OSError(f"broken table {name}")

    def missing(name):
        raise FileNotFoundError(f"classes.toml missing ({name})")

    entries = (
        RuleEntry(
            id="classes-reader",
            on=(("PreToolUse", ("Edit",)),),
            kind="gate",
            fail="closed",
            module=READER,
        ),
    )

    def fire(file_path):
        payload = helpers.payload("PreToolUse/edit-main.json")
        payload["cwd"] = str(project)
        payload["tool_input"] = {"file_path": str(file_path)}
        code, out, err = helpers.run_hook(payload, "gate", entries=entries, env=base_env(project))
        assert code == 10, (out, err)
        assert "policy_invalid" in err
        assert any(event.get("code") == "policy_invalid" for event in read_events(project))

    # A state path: the class table the runner itself reads does not load.
    monkeypatch.setattr(hooks_policy, "_load_table", broken)
    monkeypatch.setattr(hooks_policy, "_policy", None)
    fire(project / ".factory" / "specs" / "product" / "brief.md")

    # A product path: the tables load for the runner, but the load
    # path_class makes for classes.toml fails — the same refusal, not a
    # leaked FileNotFoundError (S3R-hook-runner-b-5).
    monkeypatch.setattr(hooks_policy, "_load_table", original_load)
    monkeypatch.setattr(hooks_policy, "_policy", None)
    monkeypatch.setattr(vsdd_policy, "load", missing)
    fire(project / "src" / "main.rs")
