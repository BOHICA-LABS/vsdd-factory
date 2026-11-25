"""The body of ``vsdd survey`` (commands/survey.md; plan survey.md 3.1,
decisions D2, D3, D7, D11, D13).

Built for the ``story`` workflow with ``--subject``. The order the plan
fixes: usage checks; the caller marker; every precondition, with nothing
written when one is unmet; the opening flush; ``run_begin``, which settles
the lock and writes the run record; the story's admission. The body prints
nothing and returns ``(object, code)`` — 0 or 1; a usage error is the
dispatcher's ``Refuse``, exit 2.
"""

from __future__ import annotations

from .store import root_of

__all__: list[str] = []  # the facade lists only state-api names

_BUILT_WORKFLOW = "story"


def run_survey(inv, args) -> tuple[dict, int]:
    """One whole survey; returns ``(output, exit_code)``."""
    root = root_of(inv.root)
    workflow = str(getattr(args, "workflow", ""))
    subject = str(getattr(args, "subject", "") or "")
    # --- usage checks (D3): what is not built refuses out loud, exit 2 ---
    if workflow != _BUILT_WORKFLOW:
        from ..cli.dispatch import Refuse

        raise Refuse(f"vsdd survey {workflow} is not built yet; only 'story' is")
    if not subject:
        from ..cli.dispatch import Refuse

        raise Refuse("usage: vsdd survey story --subject <story-id>")
    if getattr(args, "repo", None):
        from ..cli.dispatch import Refuse

        raise Refuse("--repo is not built yet (a hub layout); the slice is one repository")
    for flag in ("request_uri", "guidance_uri", "question_uri"):
        if getattr(args, flag, None):
            from ..cli.dispatch import Refuse

            raise Refuse(f"--{flag.replace('_', '-')} is not built yet; only 'story' takes no free text")
    if _layout(root) == "hub":
        from ..cli.dispatch import Refuse

        raise Refuse("a hub layout is not built yet; the slice is one repository")
    open_cycles = _open_cycles(root)
    if len(open_cycles) > 1:
        from ..cli.dispatch import Refuse

        raise Refuse(
            "the project has "
            + ", ".join(str(cycle.get("id", "?")) for cycle in open_cycles)
            + " open; a survey of two open cycles is not built yet"
        )
    if open_cycles and open_cycles[0].get("kind") == "hotfix":
        from ..cli.dispatch import Refuse

        raise Refuse(
            "a hotfix cycle is not built yet; its stories are cut from hotfix/<version>, "
            "not from the integration head (topics/worktrees.md)"
        )
    given_cycle = getattr(args, "cycle", None)
    if given_cycle:
        open_id = open_cycles[0].get("id", "") if open_cycles else ""
        if open_id != given_cycle:
            from ..cli.dispatch import Refuse

            raise Refuse(
                f"the one open cycle is {open_id!r}; --cycle must name it"
                if open_id
                else "no cycle is open; --cycle must name the open cycle"
            )

    cycle_id = open_cycles[0].get("id", "") if open_cycles else ""
    output = _empty(inv, workflow, subject, cycle_id)

    # --- the caller marker: only the marker is consumed here ---
    from .callers import caller

    marker = caller(inv.caller_key or "", project=root) if inv.caller_key else None
    output["next"] = _route_next(root)

    # --- preconditions (D2): unmet is ok false, every row listed, nothing written ---
    from .preconditions import check

    rows = check(
        root,
        workflow,
        subject=subject,
        cycle=cycle_id,
        hooks_alive=marker is not None,
    )
    output["preconditions"] = rows
    if marker is None:
        output["error"] = "no caller marker: the hooks are not active for this session"
        return output, 1
    unmet = [row for row in rows if not row["met"]]
    if unmet:
        output["error"] = "; ".join(
            f"{row['name']}: {row['detail']}" for row in unmet if row["detail"]
        ) or "; ".join(f"{row['name']}: unmet" for row in unmet)
        return output, 1

    # --- the opening flush (D11): failed or partial is exit 1, before any lock ---
    from .commits import flush

    flushed = flush(root, f"survey {workflow}")
    if not flushed["ok"] and flushed["state"] in ("failed", "partial"):
        output["error"] = flushed.get("error") or f"the opening flush is {flushed['state']}"
        return output, 1

    # --- the lock and the run record, through run_begin (D7) ---
    from .locks import lock_scope, run_begin

    begun = run_begin(
        root,
        scope=lock_scope(root),
        workflow=workflow,
        subject=subject,
        session=marker.get("session_id", ""),
        cycle=cycle_id,
    )
    if not begun.get("ok"):
        holder = begun.get("holder") or {}
        output["error"] = (
            "the lock is held by a live run: "
            + _holder_line(holder)
            + f" (stale at {begun.get('stale_at', '')})"
        )
        return output, 1

    run_id = (begun.get("run") or {}).get("id", "")
    try:
        from .admission import admit

        # A story in the open wave is admitted by the plan (the closure's
        # admitted_by), never by the run; a run id would mean an out-of-plan
        # admission through gate plan --story or story new-fix.
        admit(root, cycle=cycle_id, story=subject, run=run_id, admitted_by="plan")
    except Exception as exc:
        # The lock is released first, which ends the run (plan 3.1).
        release_error = _release(root, run_id)
        output["error"] = f"the story could not be admitted: {exc}"
        if release_error:
            output["error"] += f"; the lock release left a fault: {release_error}"
        return output, 1

    output["ok"] = True
    try:
        return _success(inv, root, output, begun["run"], cycle_id, subject), 0
    except Exception as exc:  # a corrupt record or a faulty gate is a fault, named
        output["ok"] = False
        output["units"] = []
        # The answer cannot be built: the lock is released first, which ends
        # the run, and exit 1 names no run (plan 3.1) — as on a failed
        # admission (finding S3R-survey-C1-1).
        release_fault = _release(root, run_id)
        output["run_id"] = ""
        output["error"] = f"the survey could not build the story unit: {exc}"
        if release_fault:
            output["error"] += f"; the lock release left a fault: {release_fault}"
        return output, 1


def _success(inv, root, output: dict, run: dict, cycle_id: str, subject: str) -> dict:
    """Every field of the survey object, real (plan 3.1)."""
    from .cycles import closure_read
    from .story_unit import build

    output["run_id"] = run.get("id", "")
    output["scope"] = "delta" if cycle_record_baseline(root, cycle_id) else "full"
    output["project_dir"] = str(root)
    output["plugin_root"] = str(inv.plugin_root)
    output["head"] = _head(root)
    closure = closure_read(root, cycle_id, "story", subject) or {}
    stages = closure.get("stages") or {}
    output["done"] = [name for name, entry in stages.items()
                      if isinstance(entry, dict) and entry.get("state") == "done"]
    config = _config(root)
    output["config"] = {
        "tier_override": _tier_override(inv),
        "review_rules": _review_rules(root),
        "holdout": _holdout(config),
    }
    output["limits"] = config.prelude_limits() if config else {}
    output["units"] = [build(root, cycle_id, subject)]
    from .decisions import decisions_list

    try:
        output["decisions"] = [qualified.split("@", 1)[0] for qualified, _ in decisions_list(root, cycle=cycle_id)]
    except Exception:
        output["decisions"] = []
    return output


def cycle_record_baseline(root, cycle_id: str) -> str:
    from .cycles import cycle_read

    cycle = cycle_read(root, cycle_id) or {}
    return str(cycle.get("baseline", "") or "")


def _holder_line(holder: dict) -> str:
    return (
        f"workflow {holder.get('workflow', '')!r}, subject {holder.get('subject', '')!r}, "
        f"run {holder.get('run', '')!r}, session {holder.get('session', '')!r}, "
        f"seen {holder.get('seen', '')!r}"
    )


def _release(root, run_id: str) -> str:
    """Release the failed run's lock; an empty answer is a release."""
    from .store import StoreBusy
    from .locks import lock_release

    try:
        lock_release(root, run=run_id)
    except StoreBusy as busy:
        kept = getattr(busy, "kept", [])
        return (
            f"the lock could not be released ({len(kept)} kept): a flush is running; "
            "the run stays live and the next survey takes it over"
        )
    except Exception as exc:
        return str(exc)
    return ""


def _empty(inv, workflow: str, subject: str, cycle_id: str) -> dict:
    """The whole survey object, empty where there is nothing to say."""
    return {
        "ok": False,
        "error": "",
        "run_id": "",
        "workflow": workflow,
        "subject": subject,
        "cycle": cycle_id,
        "scope": "",
        "project_dir": str(inv.root),
        "plugin_root": str(inv.plugin_root),
        "head": "",
        "preconditions": [],
        "done": [],
        "units": [],
        "config": {},
        "limits": {},
        "spikes_used": 0,
        "rounds_used": 0,
        "decisions": [],
        "waived": [],
        "next": {"kind": "", "command": "", "args": {}},
    }


def _open_cycles(root) -> list[dict]:
    from .cycles import cycles_open

    try:
        return cycles_open(root)
    except Exception:
        return []


def _config(root):
    from ..config import load

    try:
        return load(root / ".factory" / "factory.toml")
    except Exception:
        return None


def _layout(root) -> str:
    """The project layout from the configuration; ``single`` without one."""
    config = _config(root)
    if config is None:
        return "single"
    project = config.data.get("project") or {}
    return str(project.get("layout", "single") or "single") if isinstance(project, dict) else "single"


def _head(root) -> str:
    """The commit of the branch the run can write."""
    from . import worktrees

    config = _config(root)
    if config is None:
        return ""
    try:
        return worktrees.target_head(root, config)[1]
    except Exception:
        return ""


def _route_next(root) -> dict:
    from .route import next_step

    try:
        return next_step(root)
    except Exception:
        return {"kind": "human", "command": "", "args": {"reason": "the route could not be computed"}}


def _tier_override(inv) -> dict | None:
    """``VSDD_TEST_TIER`` is ``<model>[:<effort>]``, effort ``low`` by default
    (decision D13)."""
    raw = ""
    try:
        raw = str((inv.environ or {}).get("VSDD_TEST_TIER", "") or "")
    except Exception:
        raw = ""
    if not raw:
        return None
    model, _, effort = raw.partition(":")
    return {"model": model, "effort": effort or "low"}


def _review_rules(root) -> str:
    path = root / ".factory" / "specs" / "architecture" / "review-rules.md"
    return str(path) if path.is_file() else ""


def _holdout(config) -> dict:
    data = config.data["holdout"] if config else {}
    return {
        "mean_min": data.get("mean_min", 0.0),
        "must_pass_min": data.get("must_pass_min", 0.0),
        "stddev_max": data.get("stddev_max", 0.0),
    }
