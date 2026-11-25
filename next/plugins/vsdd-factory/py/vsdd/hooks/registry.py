"""The rule table and the ``hooks.json`` generator (plan 3.3; HD2).

HD1: a rule is one module ``vsdd/rules/<id with underscores>.py`` exposing
``check`` plus one row in :data:`RULES` — the row is the declaration. HD2: the
wiring is derived; an event is wired when a rule or a core duty names it, and
the ``PreToolUse`` matcher is the atoms they name, in :data:`TOOL_ATOMS` order.
Four duties are wired with no rule at this depth: the caller marker
(``PreToolUse`` on ``Bash``), the session-start briefing and its fixed lines
and notices (``SessionStart``), and the two lock-liveness duties of the second
part of H1 — the refresh, on every wired ``PreToolUse`` call, and the release
on ``SessionEnd``.

``hooks/hooks.json`` is generated from here by ``tools/build.py`` and checked
in; ``tools/build.py --check`` fails when the checked-in file differs. Rows sit
between the two marker comments so the record stores can inject theirs without
touching the surrounding code.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass

from .model import RuleEntry

__all__ = [
    "EVENTS",
    "TOOL_ATOMS",
    "RULES",
    "EventSpec",
    "WiringError",
    "select",
    "matches",
    "wiring",
    "hooks_json",
    "guard_command",
    "guard_body",
]


class WiringError(ValueError):
    """The wiring request cannot be generated: an unknown event, a tool that
    is no atom, or ``async`` on an event with a gate rule."""


@dataclass(frozen=True)
class EventSpec:
    """One row of [[table:events]]: the launch mode, the wired matcher, the
    platform timeout in seconds (``None`` when the platform's own budget
    governs, on ``SessionEnd``), the runner's own deadline in seconds, whether
    gate handlers carry ``onFailure: "block"``, and whether the handler is
    asynchronous."""

    name: str
    mode: str
    matcher: str | None
    timeout: int | None
    deadline: float
    on_failure: bool = False
    is_async: bool = False


#: The ordered atoms of the ``PreToolUse`` matcher; a rule names an atom by its
#: text. Matching is a whole-name regular-expression match on ``tool_name``.
#: Declared one by one and joined, never split out of the joined matcher: the
#: MCP groups contain ``|`` themselves, and splitting the joined form yields
#: fragments that are no atoms.
TOOL_ATOMS: tuple[str, ...] = (
    "Bash",
    "Edit",
    "Write",
    "MultiEdit",
    "NotebookEdit",
    "Read",
    "Grep",
    "Glob",
    "Agent",
    "Task",
    "StructuredOutput",
    "Workflow",
    "WebFetch",
    "WebSearch",
    "mcp__(plugin_vsdd-factory_)?playwright__browser_(navigate|navigate_back|tabs|file_upload)",
    "mcp__(plugin_vsdd-factory_)?(perplexity|tavily|context7)__.*",
)

#: The seven rows of tables/events.toml, in table order (HD4 fixes the
#: deadlines; the table fixes mode, matcher, timeout and onFailure).
EVENTS: tuple[EventSpec, ...] = (
    EventSpec("SessionStart", "brief", None, 15, 8.0),
    EventSpec("UserPromptExpansion", "gate", "^vsdd-factory:decide$", 10, 4.0, on_failure=True),
    EventSpec(
        "PreToolUse",
        "gate",
        "|".join(TOOL_ATOMS),
        10,
        4.0,
        on_failure=True,
    ),
    EventSpec("SubagentStart", "notify", "^vsdd-factory:|^workflow-subagent$", 5, 1.0, is_async=True),
    EventSpec("SubagentStop", "notify", "^vsdd-factory:|^workflow-subagent$", 5, 4.0),
    EventSpec("PreCompact", "notify", None, 30, 25.0),
    EventSpec("SessionEnd", "notify", None, None, 1.0),
)

_EVENT_BY_NAME = {event.name: event for event in EVENTS}

#: The core duties wired with no rule (HD2): the session start on every
#: source, the caller marker on Bash gate calls, and the lock liveness of
#: hook-runner-b (D1) on the two atoms that can carry a run's identity — a
#: ``vsdd`` call and a step's result — with the release on ``SessionEnd``.
#: ``(event, atom-or-None)``.
_DUTIES: tuple[tuple[str, str | None], ...] = (
    ("SessionStart", None),
    ("PreToolUse", "Bash"),
    ("PreToolUse", "StructuredOutput"),
    ("SessionEnd", None),
)

_DOWN_BRIEFING = (
    '{\\"continue\\":true,'
    '\\"hookSpecificOutput\\":{\\"hookEventName\\":\\"SessionStart\\",'
    '\\"additionalContext\\":\\"vsdd-factory: the plugin files are missing, so the factory gates are '
    'blocking. Reinstall the plugin.\\"},'
    '\\"systemMessage\\":\\"vsdd-factory: the plugin files are missing; every guarded call is blocked\\"}'
)

_GUARD_BODY = (
    'f="${{CLAUDE_PLUGIN_ROOT}}/py/launch.sh"; [ -r "$f" ] && exec sh "$f" hook {mode}; '
    'd="${{CLAUDE_PROJECT_DIR:-$PWD}}"; '
    'case "$d" in /*) ;; *) exit 0;; esac; '  # HD12: the walk starts only at an absolute path
    'h="${{HOME%/}}"; '  # HD12: a trailing slash on HOME must not defeat the stop
    'while [ -n "$d" ] && [ "$d" != "$h" ] && [ "$d" != / ]; do '
    '[ -f "$d/.factory/factory.toml" ] && {{ {action} ; }}; '
    'd="${{d%/*}}"; done; exit 0'
)

# The actions sit inside the single-quoted `sh -c`, so they carry no single
# quote of their own; the down-briefing's double quotes are escaped for the
# shell and its text holds no % that printf would read as a conversion.
_ACTIONS = {
    "gate": (
        'printf "vsdd-factory: the plugin files are missing, so the factory gates cannot run. '
        'Reinstall the plugin.\\n" >&2; exit 2'
    ),
    "brief": f'printf "{_DOWN_BRIEFING}\\n"; exit 0',
    "notify": "exit 0",
}


def guard_body(mode: str) -> str:
    """The body of the self-guarding command, without its ``sh -c`` wrapper:
    what :func:`guard_command` runs, exposed so a test can execute it directly
    under each interpreter instead of always through ``/bin/sh``."""
    if mode not in _ACTIONS:
        raise WiringError(f"unknown hook mode {mode!r} (gate, brief, notify)")
    return _GUARD_BODY.format(mode=mode, action=_ACTIONS[mode])


def guard_command(mode: str) -> str:
    """The self-guarding command of [[topic:hook-wiring]], per mode. When the
    launcher is readable it is exec'd; when it is missing the guard answers
    loudly in a factory project only, and stays silent otherwise (D24).
    """
    return "sh -c '" + guard_body(mode) + "'"


def select(event: str, tool: str, entries: tuple[RuleEntry, ...] | None = None) -> list[RuleEntry]:
    """The rules for one call, in table order; no rule module is imported
    here. An atom matches the whole tool name as a regular expression; a pair
    with no atoms selects every call of the event."""
    if entries is None:
        entries = RULES
    chosen = []
    for entry in entries:
        for pair_event, atoms in entry.on:
            if pair_event != event:
                continue
            if not atoms or any(matches(atom, tool) for atom in atoms):
                chosen.append(entry)
                break
    return chosen


def matches(atom: str, tool: str) -> bool:
    """Whole-name match of one tool atom against the tool name."""
    try:
        return re.fullmatch(atom, tool) is not None
    except re.error:
        return atom == tool


def _wire_entry(event: EventSpec, atoms: list[str | None], every_tool: bool) -> dict:
    """One wire: the guard command of the event's mode, the table's timeout,
    ``onFailure`` and ``async``, and the matcher the event carries. A fixed
    matcher (every wired event but ``PreToolUse``) is the event row's own;
    ``PreToolUse`` is the atoms named, in :data:`TOOL_ATOMS` order, or every
    atom when a row names none."""
    handler: dict = {"type": "command", "command": guard_command(event.mode)}
    if event.timeout is not None:
        handler["timeout"] = event.timeout
    if event.on_failure:
        handler["onFailure"] = "block"
    if event.is_async:
        handler["async"] = True
    wire: dict = {"hooks": [handler]}
    if event.name == "PreToolUse":
        if every_tool:
            wire["matcher"] = "|".join(TOOL_ATOMS)
        elif any(atom is not None for atom in atoms):
            named = [atom for atom in TOOL_ATOMS if atom in atoms]  # TOOL_ATOMS order
            wire["matcher"] = "|".join(named)
    elif event.matcher is not None:
        wire["matcher"] = event.matcher
    return wire


def wiring(entries: tuple[RuleEntry, ...] | None = None) -> dict:
    """The ``hooks.json`` document for the table: every event a rule or a core
    duty names. A fixed-matcher event is wired with its row's matcher;
    ``PreToolUse`` with the atoms named, in :data:`TOOL_ATOMS` order, and with
    every atom when any ``PreToolUse`` row names none."""
    if entries is None:
        entries = RULES
    for event in EVENTS:
        # Sanity of the table itself: an event cannot be both a gate and
        # asynchronous, because the platform ignores decisions from
        # background hooks.
        if event.is_async and event.mode == "gate":
            raise WiringError(f"{event.name}: async on a gate handler is refused")
    named: dict[str, list[str | None]] = {}
    every_tool: set[str] = set()
    for event_name, atom in _DUTIES:
        _known_event(event_name)
        bucket = named.setdefault(event_name, [])
        if atom is None:
            every_tool.add(event_name)
        elif atom not in bucket:
            bucket.append(atom)
    for entry in entries:
        for event_name, atoms in entry.on:
            event = _known_event(event_name)
            # A gate rule on an asynchronous event is refused: the platform
            # ignores decisions from background hooks, so the gate could
            # never block (HD2's refusal, tested against the real table).
            if entry.kind == "gate" and event.is_async:
                raise WiringError(
                    f"rule {entry.id}: a gate rule on {event_name} is refused: "
                    "the event is asynchronous, and the platform ignores its decisions"
                )
            bucket = named.setdefault(event_name, [])
            if not atoms:
                every_tool.add(event_name)
                continue
            for atom in atoms:
                if atom and atom not in TOOL_ATOMS:
                    raise WiringError(
                        f"rule {entry.id}: {atom!r} is not a tool atom "
                        f"(atoms: {'|'.join(TOOL_ATOMS)})"
                    )
                if event_name == "PreToolUse" and atom is not None and atom not in bucket:
                    bucket.append(atom)
    hooks: dict[str, list] = {}
    for event in EVENTS:
        if event.name in named:
            hooks[event.name] = [_wire_entry(event, named[event.name], event.name in every_tool)]
    return {"hooks": hooks}


def _known_event(name: str):
    event = _EVENT_BY_NAME.get(name)
    if event is None:
        raise WiringError(
            f"unknown event {name!r} (wired events: {', '.join(_EVENT_BY_NAME)})"
        )
    return event


def hooks_json(entries: tuple[RuleEntry, ...] | None = None) -> str:
    """The exact bytes of ``hooks/hooks.json``."""
    return json.dumps(wiring(entries), indent=2) + "\n"


#: The rule table (HD1). Rows sit between the markers; ``tools/build.py``
#: writes this generated file and the record stores inject their rows here.
RULES: tuple[RuleEntry, ...] = (
    # >>> rules
    RuleEntry(
        id="tests-first",
        on=(("PreToolUse", ("Edit", "Write", "MultiEdit", "NotebookEdit", "Bash")),),
        kind="gate",
        fail="closed",
        spike_exempt=True,
        summary=(
            "In a story, product source is written by the implementer only "
            "after red evidence exists, and by the test writer (stubs) only "
            "before; tests frozen at the red gate stay frozen for other builders"
        ),
    ),
    RuleEntry(
        id="merge-guard",
        on=(("PreToolUse", ("Bash",)),),
        kind="gate",
        fail="closed",
        summary=(
            "A direct merge into, push to, commit on or move of the "
            "integration or release branch is refused, and the governed tool "
            "command is named"
        ),
    ),
    RuleEntry(
        id="step-results",
        on=(("PreToolUse", ("StructuredOutput",)),),
        kind="gate",
        fail="closed",
        locked=True,
        rewrites=True,
        summary=(
            "Stores the step result when a workflow step returns: gate results "
            "matched against the evidence store, review findings grounded, "
            "ungrounded findings sent back once marked advisory"
        ),
    ),
    # <<< rules
)
