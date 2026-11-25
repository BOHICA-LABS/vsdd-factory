"""Loads the shipped policy tables (conventions.md, "Policy tables").

The plugin ships three tables under ``py/vsdd/policy/``:

- ``artifacts.toml`` — the artifact registry, one table per artifact kind
  (paths, identifiers, frontmatter, caps, writers, lifecycles), with the
  story-kind table (the tests stage and implement stage of each kind and
  whether it has a red gate) and the ``[_unknown]`` policy;
- ``classes.toml`` — every path class, its globs and writer, and the
  precedence among the seven product classes;
- ``walls.toml`` — one row per caller (read denials, write grants, dispatch,
  command families, vsdd groups), the ``[callers]`` subcommand groups and the
  ``[reveals]`` table.

Each is the single definition of its facts: the design files under
``rewrite/spec/tables/`` point here. Standard library only (``tomllib`` is
3.11+, the floor).
"""

from __future__ import annotations

import tomllib
from pathlib import Path
from typing import Any

__all__ = ["POLICY_DIR", "TABLES", "load", "load_all"]

POLICY_DIR = Path(__file__).resolve().parent

TABLES = ("artifacts", "classes", "walls")


def load(name: str) -> dict[str, Any]:
    """Parse one shipped table by name; ``KeyError`` for an unknown name."""
    if name not in TABLES:
        raise KeyError(f"unknown policy table {name!r}; the shipped tables are {TABLES}")
    path = POLICY_DIR / f"{name}.toml"
    with path.open("rb") as handle:
        return tomllib.load(handle)


def load_all() -> dict[str, dict[str, Any]]:
    """Parse every shipped table, keyed by name."""
    return {name: load(name) for name in TABLES}
