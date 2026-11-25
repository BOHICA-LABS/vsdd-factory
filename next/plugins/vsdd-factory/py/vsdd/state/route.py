"""The route: the rows of ``tables/routes.toml`` the slice builds, as an
ordered list of named predicate functions (plan survey.md 3.5, decision D12).

Built: rows 1 and 2 (not initialised; a live lock), rows 12 to 16 (spec,
missing commands, plan, waiting on decisions, wave) and the guard D12 adds
after row 2, which answers out loud for every state a built row does not
match. Never raises for a readable project: a predicate that cannot judge
answers False, and the walk ends at the guard's answer.
"""

from __future__ import annotations

from .store import root_of

__all__: list[str] = []  # the facade lists only state-api names

_NOT_BUILT = "this project state needs a route row that is not built yet"


def next_step(root) -> dict:
    """The first matching row, as ``{kind, command, args}``; of kind ``human``
    the ``args.reason`` is the line for the person (decision D10)."""
    root = root_of(root)
    if not _initialised(root):
        return _human("the project is not initialised; run /vsdd-factory:setup",
                      command="/vsdd-factory:setup")
    if _live_lock(root):
        return _human(
            "a run lock is held by a live run: wait, or inspect the run; a run that "
            "stopped in this session continues when its command is run again, and "
            "vsdd lock release frees its lock"
        )
    config = _config(root)
    if config is None:
        return _guard()
    try:
        layout = config.data["project"]["layout"]
    except (KeyError, TypeError):
        layout = "single"
    if layout == "hub":
        return _guard()
    if _request_queued(root):
        return _guard()
    cycles = _cycles(root)
    if len(cycles) != 1:
        return _guard()
    cycle = cycles[0]
    if cycle.get("kind") != "product" or cycle.get("route", "standard") != "standard":
        return _guard()

    # Row 12: the brief is approved and the specification is not converged,
    # or its decision asked for revision.
    if _brief_approved(root) and not _spec_ok(root):
        return _workflow("spec")
    # Row 13: the specification is approved and [commands] has no test or test_report.
    if _spec_ok(root):
        missing = _missing_commands(config)
        if missing:
            return _human(
                "the lines to add to factory.toml: [commands] "
                + " and ".join(f"{key} = <the project's {key} command>" for key in missing)
            )
        # Row 14: the plan unit is not closed.
        if not _plan_closed(root, str(cycle.get("id", ""))):
            return _workflow("plan")
        # Rows 15 and 16: the open wave.
        wave, waiting, runnable = _wave_state(root, str(cycle.get("id", "")))
        if waiting and not runnable:
            answer = _human("the decisions: " + ", ".join(dec for dec in waiting))
            answer["args"]["decisions"] = list(waiting)  # D10: the ids are listed
            return answer
        if wave:
            return _workflow("wave")  # a wave is open, or a later wave has runnable stories
    return _guard()


# --------------------------------------------------------------- helpers ---


def _human(reason: str, command: str = "") -> dict:
    return {"kind": "human", "command": command, "args": {"reason": reason}}


def _workflow(command: str) -> dict:
    return {"kind": "workflow", "command": command, "args": {}}


def _guard() -> dict:
    return _human(_NOT_BUILT)


def _initialised(root) -> bool:
    return (root / ".factory" / "factory.toml").is_file()


def _live_lock(root) -> bool:
    from .locks import lock_entries, lock_is_stale

    try:
        return any(not lock_is_stale(root, entry) for entry in lock_entries(root))
    except Exception:
        return False


def _config(root):
    from ..config import load

    try:
        return load(root / ".factory" / "factory.toml")
    except Exception:
        return None


def _request_queued(root) -> bool:
    requests = root / ".factory" / "requests"
    if not requests.is_dir():
        return False
    return any(requests.glob("*.md"))


def _cycles(root) -> list[dict]:
    from .cycles import cycles_open

    try:
        return cycles_open(root)
    except Exception:
        return []


def _decisions_of(root, kind: str) -> list[tuple[str, dict]]:
    from .decisions import decisions_list

    try:
        return decisions_list(root, kind=kind)
    except Exception:
        return []


def _latest_status(root, kind: str) -> tuple[str, str]:
    """The status of the newest decision of the kind, and its qualified id."""
    found = _decisions_of(root, kind)
    if not found:
        return "", ""
    found.sort(key=lambda pair: (pair[1].get("opened", ""), pair[0]))
    qualified, record = found[-1]
    from .decisions import decision_status

    try:
        return decision_status(root, qualified), qualified
    except Exception:
        return "", qualified


def _brief_approved(root) -> bool:
    status, _ = _latest_status(root, "brief")
    return status == "approved"


def _spec_ok(root) -> bool:
    from .decisions import spec_approved

    try:
        return bool(spec_approved(root)[0])
    except Exception:
        return False


def _missing_commands(config) -> list[str]:
    commands = config.data.get("commands", {}) if hasattr(config, "data") else {}
    return [key for key in ("test", "test_report") if not commands.get(key)]


def _plan_closed(root, cycle: str) -> bool:
    from .cycles import closure_read

    closure = closure_read(root, cycle, "plan", "")
    return closure is not None and closure.get("state") == "closed"


def _wave_state(root, cycle: str) -> tuple[str, list[str], list[str]]:
    """``(open wave, open decisions, runnable stories)`` — the inputs of rows
    15 and 16 (D12). The wave is open while its closure is not ``closed``,
    whatever its stories' deliveries; the wave row owes a run of the wave's
    gate even after every story of the wave is delivered."""
    from .admission import open_wave
    from .cycles import closure_read
    from .story_unit import position

    wave = open_wave(root, cycle)
    if not wave:
        return "", [], []
    plan = closure_read(root, cycle, "plan", "") or {}
    stories = ((plan.get("waves") or {}).get(wave)) or []
    if not isinstance(stories, list):
        return wave, [], []
    waiting: list[str] = []
    runnable: list[str] = []
    for story in stories:
        story = str(story)
        if position(root, cycle, story) == "delivered":
            continue
        holders = _open_decisions_on(root, story)
        if holders:
            waiting.extend(holders)
            continue
        if _could_run(root, cycle, story):
            runnable.append(story)
    return wave, waiting, runnable


def _could_run(root, cycle: str, story: str) -> bool:
    """The route's "could run" test, the same ``preconditions.check`` the
    survey answers on with ``hooks_alive=None``, which leaves that row out."""
    from .preconditions import check

    try:
        rows = check(root, "story", subject=story, cycle=cycle, hooks_alive=None)
    except Exception:
        return False
    return all(row["met"] for row in rows)


def _open_decisions_on(root, story: str) -> list[str]:
    """Qualified ids of open decisions whose subject is ``story``."""
    from .decisions import decisions_list, decision_status

    found: list[str] = []
    try:
        pairs = decisions_list(root)
    except Exception:
        return []
    for qualified, record in pairs:
        ident = qualified.split("@", 1)[0]
        subject = ident.split(".", 1)[1] if "." in ident else ""
        if subject != story:
            continue
        try:
            if decision_status(root, qualified) == "open":
                found.append(qualified)
        except Exception:
            continue
    return found
