"""The one writer of patch files (plan gates.md 3.5; ORDER.md section 3, "A
patch file").

Green and delivery write patches no other way. The name says the range:
``diff.<head12>.patch`` for the story's base to its head, ``diff.<since12>-<head12>.patch``
for a ``--since`` range, so a narrowed patch never replaces the whole patch
the next run's full review is given (finding PL4-6).
"""

from __future__ import annotations

import os
from pathlib import Path

from ..state.gate_evidence import evidence_dir

__all__ = ["write_patch"]


def write_patch(project, subject: str, base: str, head: str, *, tree, since: str = "") -> str:
    """Write ``git diff --no-color --no-ext-diff <since or base> <head>`` of
    ``tree`` into the subject's evidence directory, by rename, and return the
    absolute path. An empty diff is an empty file; :class:`GitError` for an
    unknown commit. Two writers at once never collide: the temporary file is
    the store's, unique per writer, and the rename is atomic."""
    from ..state import gitrun
    from ..state.store import write_via_tmp

    tree = Path(tree)
    start = since or base
    stem = f"{head[:12]}.patch" if not since else f"{since[:12]}-{head[:12]}.patch"
    directory = evidence_dir(project, subject)
    directory.mkdir(parents=True, exist_ok=True)
    target = directory / f"diff.{stem}"
    diff = gitrun.git(tree, "diff", "--no-color", "--no-ext-diff", start, head)
    write_via_tmp(target, diff.encode("utf-8"))
    return str(target if target.is_absolute() else os.path.abspath(target))
