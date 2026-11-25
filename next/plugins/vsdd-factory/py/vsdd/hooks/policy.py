"""The policy tables as the rules read them (plan 3.6; hooks.md 2.7).

One loader, one cache: :func:`tables` returns the wall table and the class
table parsed once per process, and resolves an ``agent_type`` to its
[[table:walls]] row. A table that is missing or does not parse raises
:class:`PolicyInvalid`, which the caller turns into the ``policy_invalid``
failure of the rule that read it — a missing table relaxes nothing.
"""

from __future__ import annotations

from dataclasses import dataclass

from ..policy import load as _load_table

__all__ = ["PolicyInvalid", "PolicyRow", "Policy", "tables"]


class PolicyInvalid(RuntimeError):
    """A policy table is missing or invalid; the rules that need it fail
    closed with the ``policy_invalid`` code."""


@dataclass(frozen=True)
class PolicyRow:
    """One row of the wall table: its name (``main``, ``other`` or the agent
    type), its raw data, and whether it is the spike row."""

    name: str
    data: dict
    spike: bool

    def get(self, key: str, default=None):
        return self.data.get(key, default)


@dataclass(frozen=True)
class Policy:
    """The parsed tables."""

    walls: dict
    classes: dict

    def row(self, agent_type: str) -> PolicyRow:
        """The row for one ``agent_type``: the agent's own row, ``main`` for
        the main session (an empty type), ``other`` for any type that is not
        listed — ``workflow-subagent`` is ``other``."""
        walls = self.walls
        if agent_type and agent_type in walls and isinstance(walls[agent_type], dict):
            data = walls[agent_type]
            return PolicyRow(agent_type, data, bool(data.get("spike")))
        if not agent_type or agent_type == "main":
            return PolicyRow("main", _as_table(walls, "main"), bool(_as_table(walls, "main").get("spike")))
        return PolicyRow("other", _as_table(walls, "other"), bool(_as_table(walls, "other").get("spike")))


def _as_table(walls: dict, name: str) -> dict:
    row = walls.get(name)
    return row if isinstance(row, dict) else {}


_policy: Policy | None = None


def tables() -> Policy:
    """Parse and cache the shipped tables; :class:`PolicyInvalid` when one is
    missing or does not parse."""
    global _policy
    if _policy is None:
        try:
            walls = _load_table("walls")
            classes = _load_table("classes")
        except (OSError, KeyError, ValueError) as exc:
            raise PolicyInvalid(f"a policy table is missing or invalid: {exc}") from exc
        if not isinstance(walls, dict) or not isinstance(classes, dict):
            raise PolicyInvalid("a policy table is not a table")
        _policy = Policy(walls=walls, classes=classes)
    return _policy
