"""The survey's preconditions (tables/preconditions.toml; plan survey.md 3.1,
decision D2).

One function per predicate and ``check`` over them, in the order the plan
fixes, evaluated before anything is written — an unmet precondition stops the
survey with every row listed and only the caller marker consumed. The route's
"could run" test is the same ``check`` with ``hooks_alive=None``, which leaves
that row out.
"""

from __future__ import annotations

from .store import root_of

__all__: list[str] = []  # the facade lists only state-api names


def check(root, workflow: str, *, subject: str = "", cycle: str = "", hooks_alive=None) -> list[dict]:
    """Every precondition of ``workflow`` as ``{name, met, detail}`` rows.

    ``hooks_alive=None`` leaves that row out — the route's "could run" form.
    Only ``story`` is built; another workflow gets the common rows.
    """
    root = root_of(root)
    rows = [_project_ready(root)]
    if hooks_alive is not None:
        rows.append(
            {
                "name": "hooks_alive",
                "met": bool(hooks_alive),
                "detail": "" if hooks_alive else "no caller marker: the hooks are not active for this session",
            }
        )
    if workflow == "story":
        ok, detail = _spec_approved(root)
        rows.append({"name": "spec_approved", "met": ok, "detail": detail})
        ok, detail = admissible_row(root, cycle, subject)
        rows.append({"name": "story_admitted", "met": ok, "detail": detail})
        ok, detail = _dependencies(root, cycle, subject)
        rows.append({"name": "dependencies_delivered", "met": ok, "detail": detail})
        ok, detail = _commands(root)
        rows.append({"name": "commands_configured", "met": ok, "detail": detail})
    return rows


def _project_ready(root) -> dict:
    from .mount import detect

    condition = detect(root)
    met = condition == "ready"
    detail = "" if met else f"the project is {condition}"
    return {"name": "project_ready", "met": met, "detail": detail}


def _spec_approved(root) -> tuple[bool, str]:
    from .decisions import spec_approved

    try:
        return spec_approved(root)
    except Exception as exc:  # a corrupt record is an unmet row, not a crash
        return False, f"the decisions cannot be read: {exc}"


def admissible_row(root, cycle: str, story: str) -> tuple[bool, str]:
    if not cycle:
        return False, "no cycle is open"
    from .admission import admissible

    try:
        return admissible(root, cycle, story)
    except Exception as exc:
        return False, f"the story cannot be judged: {exc}"


def _dependencies(root, cycle: str, story: str) -> tuple[bool, str]:
    """The closure of every ``depends_on`` story holds ``delivery.merged_sha``."""
    if not cycle:
        return False, "no cycle is open"
    from .artifacts import ArtifactError, load_artifact
    from .cycles import closure_read

    try:
        meta = load_artifact(root, story).frontmatter
    except ArtifactError:
        return True, ""  # the missing file is story_admitted's refusal to name
    waiting = []
    for dep in meta.get("depends_on") or []:
        dep = str(dep)
        closure = closure_read(root, cycle, "story", dep)
        if closure is None or not (closure.get("delivery") or {}).get("merged_sha"):
            waiting.append(dep)
    if waiting:
        return False, "waiting on " + ", ".join(waiting)
    return True, ""


def _commands(root) -> tuple[bool, str]:
    from ..config import ConfigError, load

    try:
        config = load(root / ".factory" / "factory.toml")
        commands = config.data["commands"]
    except ConfigError as exc:
        return False, f"the configuration does not load: {exc}"
    missing = [key for key in ("test", "test_report") if not commands.get(key)]
    if missing:
        return False, "[commands] is missing " + ", ".join(missing)
    return True, ""
