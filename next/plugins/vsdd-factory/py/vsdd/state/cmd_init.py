"""``vsdd init``: create or mount the state branch and write factory.toml by
detection (command:init; state-core.md, decision 3).

The first commit is built with plumbing (``hash-object``, ``mktree`',
``commit-tree``, ``update-ref`` with an empty old value), so the person's
checkout, index and hooks are untouched. Converting pre-1.0 state is the
importer's job: init reports ``legacy`` and converts nothing. Repair of a
broken mount is ``/vsdd-factory:status --fix``; init reports a ``partial``
project with that as its next step.

Every step is safe to re-run: the exclusions are written before the mount and
again on the ready path, a mount killed halfway is finished (``worktree
prune``, the interrupted-pointer cleanup), and the state branch is created
with an empty old value, so a second concurrent init loses the update and
takes the mount path instead of replacing the branch.
"""

from __future__ import annotations

from pathlib import Path

from ..cli.dispatch import Command, Reply, Refuse
from .gitrun import GitError, git
from .mount import STATE_BRANCH, _is_interrupted_mount, detect, ensure_excludes, mount_state
from .detection import render_config


def _configure(parser) -> None:
    parser.add_argument(
        "--auto-allow",
        choices=("off", "core", "extended"),
        default="off",
        help="the person's answer to decision D4, written once",
    )


def _identity_ok(root: Path) -> bool:
    email = git(root, "config", "user.email", check=False).strip()
    name = git(root, "config", "user.name", check=False).strip()
    return bool(email) and bool(name)


def _branch_tip(root: Path) -> str | None:
    """Where the state branch lives: the local name, else ``origin``'s."""
    if git(root, "show-ref", "--verify", f"refs/heads/{STATE_BRANCH}", check=False).strip():
        return STATE_BRANCH
    if git(root, "show-ref", "--verify", f"refs/remotes/origin/{STATE_BRANCH}", check=False).strip():
        return f"origin/{STATE_BRANCH}"
    return None


def _branch_tree(root: Path, tip: str) -> list[str]:
    return git(root, "ls-tree", "-r", "--name-only", tip, check=False).splitlines()


def _legacy_fields(root: Path) -> Reply:
    return _fields(
        root,
        "legacy",
        False,
        False,
        {},
        [],
        ["vsdd import --dry-run", "vsdd import"],
    )


def _create_branch(root: Path, config_text: str) -> bool:
    """The orphan first commit, built with plumbing, on the state branch.

    The update carries an empty old value (decision 3): when another init has
    created the branch meanwhile, ours fails and this run mounts theirs.
    Returns ``True`` when this run created the branch.
    """
    toml_sha = git(root, "hash-object", "-w", "--stdin", input=config_text)
    notes_sha = git(root, "hash-object", "-w", "--stdin", input='{"notes": []}')
    ignore_sha = git(root, "hash-object", "-w", "--stdin", input="local/\n")
    tree = git(
        root,
        "mktree",
        input=(
            f"100644 blob {ignore_sha}\t.gitignore\n"
            f"100644 blob {toml_sha}\tfactory.toml\n"
            f"100644 blob {notes_sha}\tnotes.json\n"
        ),
    )
    commit = git(
        root,
        "commit-tree",
        tree,
        "--no-gpg-sign",
        "-m",
        "vsdd: initialise the factory state branch",
    )
    try:
        git(root, "update-ref", f"refs/heads/{STATE_BRANCH}", commit, "")
    except GitError:
        return False  # a concurrent init created it first; mount what it wrote
    return True


def _fields(root: Path, state: str, created: bool, mounted: bool, detected, undetected, next_steps) -> Reply:
    return Reply(
        ok=state == "ready",
        code=0 if state == "ready" else 1,
        data={
            "state": state,
            "created": created,
            "mounted": mounted,
            "root": str(root),
            "config": str(Path(root) / ".factory" / "factory.toml"),
            "detected": detected,
            "undetected": undetected,
            "next": next_steps,
        },
    )


def _run(inv, args) -> Reply:
    root = inv.start
    git_path = root / ".git"
    if not git_path.exists():
        raise Refuse("not a git repository; run git init first")
    if git_path.is_file():
        raise Refuse(f"{root} is a linked worktree; run vsdd init in the main checkout")
    if not _identity_ok(root):
        raise Refuse(
            "git has no identity (user.email, user.name); configure it in your own terminal"
        )

    condition = detect(root)
    if condition == "legacy":
        return _legacy_fields(root)
    if condition == "partial":
        return _fields(
            root,
            "partial",
            False,
            False,
            {},
            [],
            ["/vsdd-factory:status --fix"],
        )
    if condition == "ready":
        # A second run changes nothing but finishes its duties: a run killed
        # between the mount and the exclusions lands here too.
        ensure_excludes(root)
        return _fields(root, "ready", False, False, {}, [], [])

    # A .factory that is anything but an empty directory or a verified
    # interrupted mount is refused before anything is created, for both
    # `none` and `unmounted` (findings S3R-4 and S3R-state-core-C1-4): a
    # refusal must not leave a freshly created state branch behind, and a
    # plain file there used to die later in `worktree add` with exactly that
    # debris.
    state_dir = root / ".factory"
    if state_dir.exists() or state_dir.is_symlink():
        empty_dir = state_dir.is_dir() and not any(state_dir.iterdir())
        if not empty_dir and not _is_interrupted_mount(state_dir):
            raise Refuse(
                f"{root}/.factory exists and is not empty, but holds no state; "
                "move it aside and run vsdd init again"
            )

    tip = _branch_tip(root)
    created = False
    detected, undetected = {}, []
    if tip is None:
        if condition == "unmounted":
            raise Refuse(
                "no state branch exists although the files said there was one; "
                "run vsdd init again"
            )
        config_text, detected, undetected = render_config(root, args.auto_allow)
        created = _create_branch(root, config_text)
        if not created:
            # The concurrent init won the branch; mount what it wrote.
            tip = _branch_tip(root)
            names = _branch_tree(root, tip)
            if "factory.toml" not in names:
                raise Refuse(
                    f"the state branch {tip} holds no factory.toml; "
                    "delete it and run vsdd init again"
                )
    else:
        names = _branch_tree(root, tip)
        # An unmounted legacy branch is reported, never mounted: conversion
        # is the importer's job (finding S3R-5).
        if "STATE.md" in names and "factory.toml" not in names:
            return _legacy_fields(root)
        if "factory.toml" not in names:
            raise Refuse(
                f"the state branch {tip} holds neither factory.toml nor STATE.md; "
                "delete it and run vsdd init again"
            )
    ensure_excludes(root)
    mount_state(root)
    ensure_excludes(root)
    state = detect(root)
    return _fields(root, state, created, True, detected, undetected, [])


COMMANDS = (
    Command(
        words=("init",),
        summary="create or mount the state branch and write factory.toml by detection",
        run=_run,
        configure=_configure,
        needs="none",
    ),
)
