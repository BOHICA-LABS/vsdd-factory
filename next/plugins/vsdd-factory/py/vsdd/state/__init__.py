"""The ``vsdd.state`` facade (state-core.md 3.2).

A state module publishes a flat ``vsdd.state.<name>`` by listing it in a
literal top-level ``__all__`` of its own module. Three rules:

- ``__all__`` holds the names of spec ``topics/state-api.md`` that the module
  implements and no other. Any other name is imported from its submodule.
- A module is never named after a name any module exports (``flush`` cannot
  live in ``flush.py``): Python would bind the submodule over the function.
- Code on the hook path imports the submodule itself
  (``from vsdd.state.locks import lock_seen``).

``import vsdd.state`` imports no submodule: the first ``vsdd.state.<name>``
builds the name table once per process, reading each sibling's ``__all__``
with a regular expression (no import, no ``ast``; modules starting ``_`` or
``cmd_`` are skipped), then imports only the owner and caches the function.
No owner, or two, raises ``AttributeError`` naming the modules.
"""

from __future__ import annotations

import importlib
import re
from pathlib import Path

_HERE = Path(__file__).resolve().parent

_ALL_RE = re.compile(r"^__all__\s*=\s*\[(.*?)\]", re.M | re.S)
_LIT_RE = re.compile(r"\"([^\"]*)\"|'([^']*)'")

_TABLE: dict[str, list[str]] | None = None


def _scan() -> dict[str, list[str]]:
    """Map every flat name to the modules exporting it, without importing."""
    table: dict[str, list[str]] = {}
    for path in sorted(_HERE.glob("*.py")):
        stem = path.stem
        if stem.startswith("_") or stem.startswith("cmd_"):
            continue
        text = path.read_text(encoding="utf-8")
        block = _ALL_RE.search(text)
        if block is None:
            continue
        for first, second in _LIT_RE.findall(block.group(1)):
            name = first or second
            table.setdefault(name, []).append(stem)
    return table


def __getattr__(name: str):
    global _TABLE
    if _TABLE is None:
        _TABLE = _scan()
    owners = _TABLE.get(name)
    if not owners:
        raise AttributeError(
            f"vsdd.state has no flat name {name!r}; the state API names are in "
            "spec topics/state-api.md and each module's own __all__"
        )
    if len(owners) > 1:
        raise AttributeError(
            f"vsdd.state flat name {name!r} is exported by more than one module: "
            + ", ".join(f"vsdd/state/{owner}.py" for owner in owners)
        )
    module = importlib.import_module(f"{__name__}.{owners[0]}")
    try:
        value = getattr(module, name)
    except AttributeError:
        raise AttributeError(
            f"vsdd/state/{owners[0]}.py lists {name!r} in __all__ but does not define it"
        ) from None
    globals()[name] = value
    return value
