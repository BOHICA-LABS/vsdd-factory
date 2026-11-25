"""The tool's one glob semantics (state-core.md 3.4).

``*`` and ``?`` do not cross ``/``; ``**`` does. Python 3.11 has no such
matcher built in. Used by ``vsdd.project.path_class``, the artifact registry,
the hook runner's path classes and the digests' file sets: one translation,
cached, so every consumer agrees.
"""

from __future__ import annotations

import functools
import re

__all__ = ["glob_regex"]


@functools.lru_cache(maxsize=None)
def glob_regex(glob: str) -> re.Pattern:
    """Compile one glob into an anchored regular expression.

    ``**`` spans directories; written as ``**/`` it also matches zero
    directories, so ``**/tests.rs`` matches a root-level ``tests.rs`` and
    ``a/**/b`` matches ``a/b`` as well as ``a/x/y/b``.
    """
    parts: list[str] = []
    i = 0
    n = len(glob)
    while i < n:
        char = glob[i]
        if char == "*":
            star_end = i
            while star_end < n and glob[star_end] == "*":
                star_end += 1
            if star_end - i >= 2:
                if star_end < n and glob[star_end] == "/":
                    parts.append("(?:.*/)?")  # **/ also matches zero directories
                    i = star_end + 1
                    continue
                parts.append(".*")
            else:
                parts.append("[^/]*")
            i = star_end
        elif char == "?":
            parts.append("[^/]")
            i += 1
        else:
            parts.append(re.escape(char))
            i += 1
    return re.compile("".join(parts) + r"\Z")
