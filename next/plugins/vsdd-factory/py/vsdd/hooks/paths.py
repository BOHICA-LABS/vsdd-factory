"""Path classes of a call's target (plan hook-runner-b 3.1; [[table:path-classes]]).

``classes_of`` answers "which classes contain this path". One function, used
by ``Ctx.classes`` and ``Ctx.classes_of``; no second precedence list and no
second glob matcher: the state-directory classes are matched with
:func:`vsdd.globs.glob_regex` (S1's one translation), the product classes come
from :func:`vsdd.project.path_class` (S1's one precedence resolver), and the
computed classes are the four of ``classes.toml``'s closing comment.

Class matching tests both forms of the path, as [[topic:rule-model]] 2.5
says: the literal form, absolute and anchored under the project, and the
symlink-resolved form. The answer is their union, the literal form's classes
first, so a write through a symlink is judged by its destination and by the
place the call names.

The regions are separate, as ``classes.toml``'s closing comment has it: per
form, a path of the project (state classes, or the product class and
``project``) is never also ``tmp``, ``tmp`` only for a path outside the
project, and ``outside`` only for a path under none of the project, the
plugin directory and the temporary directory. The union keeps every class of
both forms (S3R-hook-runner-b-C1-1): an in-project symlink whose destination
lies outside them all is ``project`` by its spelling and ``outside`` by its
destination, and both are true — a wall that admits ``project`` must not
wave the destination through.

Two ``own-worktree`` gaps are deliberate at this depth (S3R-hook-runner-b-11):
:func:`vsdd.project.containing_worktree` answers ``None`` outside
``.worktrees/``, so the platform's throwaway worktree is never
``own-worktree``, and the union with the worktree a spike runner's ledger
entry records arrives with spike-bounds (H8), which builds and tests both.

A policy table that does not load raises :class:`hooks.policy.PolicyInvalid`
on every branch: the state branch reads the table here, and the product
branch's ``path_class`` loads ``classes.toml`` itself, so its faults are
re-raised as ``PolicyInvalid`` too — ``policy_invalid`` for the rule that
read the field ([[topic:hook-failure]]).
"""

from __future__ import annotations

import os
import tempfile
from pathlib import Path

from ..globs import glob_regex
from .policy import PolicyInvalid, tables

__all__ = ["classes_of"]


def classes_of(path, project, where=None, plugin_root: str = "") -> tuple[str, ...]:
    """The classes of ``path`` in ``project``: the matched classes, then the
    computed ones, of the literal and the resolved form of the path taken
    together. ``project`` is the context's project (a
    :class:`vsdd.project.Project`); ``where`` is the call's effective
    directory for ``own-worktree``; ``plugin_root`` is the platform's
    ``CLAUDE_PLUGIN_ROOT`` when it gave one."""
    if project is not None:
        tables()  # a table that does not load is a refusal on every branch
    forms = _forms(Path(os.path.abspath(path)), project)
    names = _classify(forms[0], project, where, plugin_root)
    for form in forms[1:]:
        names = _union(names, _classify(form, project, where, plugin_root))
    return tuple(names)


def _forms(literal: Path, project) -> list[Path]:
    """The forms of the path to classify, literal leaning first: the path as
    the call spells it, anchored under the project when the spelling allows,
    and its resolved form. Without a project, or when both forms agree, only
    the resolved form is classified."""
    anchored = _anchored(literal, project)
    resolved = Path(os.path.realpath(literal))
    if anchored is None or anchored == resolved:
        return [resolved]
    return [anchored, resolved]


def _anchored(literal: Path, project) -> Path | None:
    """The literal path beneath the resolved project root, when its spelling
    allows: a symlink at the leaf stays literal — that is the form whose
    classes say where the call names the file — and a spelling through a
    symlinked ancestor of the root (macOS spells ``/var`` for the resolved
    ``/private/var``; the session root itself is resolved, so the ancestry is
    collapsed first) anchors at the resolved parent
    (S3R-hook-runner-b-6). ``None`` when the literal relates to no project
    root, not even by resolved ancestry: the resolved form alone carries the
    answer."""
    if project is None:
        return None
    root = Path(os.path.realpath(project.root))
    if _under(literal, root):
        return literal
    spelled = Path(os.path.abspath(project.root))
    if spelled != root and _under(literal, spelled):
        return root.joinpath(literal.relative_to(spelled))
    anchored = Path(os.path.realpath(literal.parent)) / literal.name
    return anchored if _under(anchored, root) else None


def _classify(abs_path: Path, project, where, plugin_root: str) -> list[str]:
    """The classes of one absolute path form: the matched classes, then the
    computed ones. ``project`` first, then ``tmp``, then ``outside``; a path
    of the project is never ``tmp`` and never ``outside``."""
    names: list[str] = []
    root = Path(os.path.realpath(project.root)) if project is not None else None
    if root is not None:
        in_state = _under(abs_path, root / ".factory")
        in_project = _under(abs_path, root)
        if in_state:
            names.extend(_state_classes(abs_path, root))
        else:
            names.extend(_product_classes(abs_path, project, root))
            if in_project:
                names.append("project")
        if _in_own_worktree(abs_path, root, where):
            names.append("own-worktree")
        if in_project:
            return names
    if _under(abs_path, _tmp_root()):
        names.append("tmp")
    elif not _under_any(abs_path, project, plugin_root, root):
        names.append("outside")
    return names


def _union(literal_names: list[str], resolved_names: list[str]) -> tuple[str, ...]:
    """Both forms taken together: the spelling-anchored form's order first,
    then the resolved form's additions. Every class of both forms is kept
    (S3R-hook-runner-b-C1-1) — dropping ``outside`` when a containing region
    is present would erase the destination of an in-project symlink that
    points outside, whose real answer is both ``project`` and ``outside``."""
    merged: list[str] = []
    for name in literal_names + resolved_names:
        if name not in merged:
            merged.append(name)
    return tuple(merged)


def _tmp_root() -> Path:
    """The temporary directory, resolved. ``gettempdir()`` answers the
    unresolved form — on macOS ``/var/folders/.../T``, under the ``/var``
    symlink — while every path this module classifies is resolved
    (S3R-hook-runner-b-3)."""
    return Path(os.path.realpath(tempfile.gettempdir()))


def _state_classes(resolved: Path, root: Path) -> list[str]:
    """Every class of ``classes.toml`` whose globs match the path relative to
    the project root, innermost first. Specificity is the number of literal
    (wildcard-free) path components of the deepest matching glob: a contained
    class's glob reaches one component deeper than its container's."""
    relative = resolved.relative_to(root).as_posix()
    matched: list[tuple[int, str]] = []
    for name, spec in tables().classes.items():
        if not isinstance(spec, dict):
            continue
        globs = spec.get("globs")
        if not isinstance(globs, list):
            continue
        depth = max(
            (
                _specificity(str(glob))
                for glob in globs
                if glob_regex(str(glob)).fullmatch(relative)
            ),
            default=-1,
        )
        if depth >= 0:
            matched.append((-depth, name))
    matched.sort()
    return [name for _, name in matched]


def _product_classes(resolved: Path, project, root: Path) -> list[str]:
    """The product class by S1's precedence, or ``unclassified``. A fault of
    the table ``path_class`` loads is a refusal, not a crash (the finding
    S3R-hook-runner-b-5 probe: ``FileNotFoundError: classes.toml missing``)."""
    from ..project import path_class

    try:
        product = path_class(resolved, root=root, config=project.config)
    except (OSError, ValueError) as exc:  # FileNotFoundError, TOMLDecodeError
        raise PolicyInvalid(f"the class table does not load: {exc}") from exc
    return [product if product is not None else "unclassified"]


def _specificity(glob: str) -> int:
    """Literal path components of one glob: ``.factory/specs/product/brief.md``
    is 4, ``.factory/specs/product/**`` is 3, ``.factory/cycles/*/decisions/**``
    is 3."""
    return sum(
        1
        for part in glob.split("/")
        if part and not (set(part) & set("*?"))
    )


def _in_own_worktree(resolved: Path, root: Path, where) -> bool:
    """``own-worktree``: inside the worktree that holds ``where``. Never the
    main checkout: ``containing_worktree`` answers ``None`` outside
    ``.worktrees/``."""
    if not where:
        return False
    from ..project import containing_worktree

    worktree = containing_worktree(Path(os.path.realpath(where)), root=root)
    if worktree is None:
        return False
    return _under(resolved, Path(os.path.realpath(worktree.path)))


def _under(path: Path, base: Path) -> bool:
    try:
        path.relative_to(base)
    except ValueError:
        return False
    return True


def _under_any(path: Path, project, plugin_root: str, root: Path | None) -> bool:
    """Whether the path is inside something named by ``outside``'s complement:
    the project, the plugin directory or the temporary directory (the project
    and tmp are checked by the caller)."""
    if root is not None and _under(path, root):
        return True
    if plugin_root and _under(path, Path(os.path.realpath(plugin_root))):
        return True
    return _under(path, _tmp_root())
