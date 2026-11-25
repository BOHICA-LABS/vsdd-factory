"""The digest an approval is bound to (topics/digests.md).

SHA-256 over the sorted lines ``<path>\\t<sha256 of the file's bytes>\\n``, cut
to 16 hex characters. ``root`` is required: a tracked digest must not depend
on where the clone sits. A ``spec`` decision covers the three specification
classes except ``brief`` and ``guidance``, so drafting a brief or editing the
guidance never makes a ``spec`` approval stale.
"""

from __future__ import annotations

import hashlib
from pathlib import Path

from .store import StoreRefused, root_of

__all__ = ["digest"]  # the facade lists only state-api names

DIGEST_LENGTH = 16


def digest(paths, *, root) -> str:
    """The digest of exactly these files, as they are.

    ``paths`` are root-relative POSIX paths, sorted here; a missing file
    refuses ``digest_missing_file``. Nothing is left out of a file: approval
    changes no file, so the whole bytes are hashed.
    """
    root = Path(root)
    lines: list[str] = []
    for path in sorted({str(p).replace("\\", "/") for p in paths}):
        file = root / path
        try:
            data = file.read_bytes()
        except OSError as exc:
            raise StoreRefused("digest_missing_file", f"{path}: cannot be read: {exc}") from exc
        lines.append(f"{path}\t{hashlib.sha256(data).hexdigest()}\n")
    whole = hashlib.sha256("".join(lines).encode("utf-8")).hexdigest()
    return whole[:DIGEST_LENGTH]


# The specification classes and the two nested classes a spec decision leaves
# out; the globs are the shipped classes table's own (py/vsdd/policy/classes.toml),
# resolved through the same glob semantics every consumer shares.
_SPEC_CLASSES = ("spec-product", "spec-arch", "spec-ux")
_OUTSIDE_CLASSES = ("brief", "guidance")


def spec_file_set(project) -> list[str]:
    """The sorted paths of the specification file set: every file in the three
    specification classes except the classes ``brief`` and ``guidance``."""
    from ..globs import glob_regex
    from ..policy import load as load_policy

    root = root_of(project)
    classes = load_policy("classes")
    outside = [
        glob_regex(glob)
        for name in _OUTSIDE_CLASSES
        for glob in classes.get(name, {}).get("globs", [])
    ]
    inside = [
        glob_regex(glob) for name in _SPEC_CLASSES for glob in classes.get(name, {}).get("globs", [])
    ]
    base = root / ".factory"
    if not base.is_dir():
        return []
    found: list[str] = []
    for path in sorted(base.rglob("*")):
        if not path.is_file():
            continue
        rel = path.relative_to(root).as_posix()
        if any(pattern.fullmatch(rel) for pattern in outside):
            continue
        if any(pattern.fullmatch(rel) for pattern in inside):
            found.append(rel)
    return found


def spec_digest(project) -> tuple[str, list[str]]:
    """The digest of the specification file set and the set itself, as a
    ``spec`` decision stores them."""
    paths = spec_file_set(project)
    return digest(paths, root=root_of(project)), paths
