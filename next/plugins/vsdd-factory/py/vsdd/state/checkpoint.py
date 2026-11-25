"""The body of ``vsdd checkpoint`` (commands/checkpoint.md; plan survey.md
3.2, decisions D1, D8, D9, D18, D19).

Built for the ``story`` workflow. The checkpoint closes the run's stories as
far as their positions reach, stops their products, flushes, and — in
``finally`` — writes the agent count and releases the lock, whatever
happened. ``--outcome failed`` closes the run as failed; every other word
changes nothing, and an unknown word is exit 2. A run whose ``ended`` is set
writes nothing: the relay retries a lost answer, and a taken-over run must
not be touched a second time.

Until the second part maps step records, stage entries come from positions
(D9), with ``position()``'s own mapping; both are provisional.
"""

from __future__ import annotations

from .store import root_of

__all__: list[str] = []  # the facade lists only state-api names

_BUILT_WORKFLOW = "story"

#: ``--outcome`` words that only confirm the computed status (plan D1).
_INERT_OUTCOMES = ("done", "partial", "blocked", "needs_human")

#: The stage entry each position records, until the second part replaces the
#: mapping with the one of step records (D9).
_STAGE_OF_POSITION = {"red": "tests", "green": "implement", "reviewed": "review", "evidenced": "evidence"}
_POSITION_ORDER = ("ready", "red", "green", "reviewed", "evidenced", "delivered")


def run_checkpoint(inv, args) -> tuple[dict, int]:
    """One whole checkpoint; returns ``(output, exit_code)``."""
    from ..cli.dispatch import Refuse
    from .runs import run_read
    from .store import StoreRefused

    root = root_of(inv.root)
    workflow = str(getattr(args, "workflow", ""))
    run_id = str(getattr(args, "run", "") or "")
    subject = str(getattr(args, "subject", "") or "")
    outcome = str(getattr(args, "outcome", "") or "")

    if workflow != _BUILT_WORKFLOW:
        raise Refuse(f"vsdd checkpoint {workflow} is not built yet; only 'story' is")
    if outcome and outcome != "failed" and outcome not in _INERT_OUTCOMES:
        raise Refuse(
            f"--outcome {outcome!r} is not a status word (failed, done, partial, "
            "blocked, needs_human)"
        )
    try:
        record = run_read(root, run_id)
    except StoreRefused as exc:  # an unusable run id is the unknown-run refusal
        raise Refuse(f"no run {run_id!r}: {exc}") from exc
    if record is None:
        raise Refuse(f"no run {run_id!r}")
    if record.get("workflow", "") != workflow:
        raise Refuse(f"run {run_id!r} is a {record.get('workflow', '')!r} run, not {workflow!r}")
    if subject and record.get("subject", "") != subject:
        raise Refuse(
            f"run {run_id!r} carries subject {record.get('subject', '')!r}, not {subject!r}"
        )
    cycle_id = record.get("cycle", "")
    stories = [str(story) for story in record.get("stories", [])]

    if record.get("ended", ""):
        # D8: an ended run is written nothing; the closures give the status.
        delivered = [_delivered(root, cycle_id, story) for story in stories]
        return {
            "ok": True,
            "error": "",
            "status": "done" if stories and all(delivered) else "partial",
            "progress": False,
            "decisions": [],
            "next": _route_next(root),
        }, 0

    errors: list[str] = []
    closure_changed = False
    push_gap = ""
    try:
        # The product stops, every close and the flush are faults, not escapes:
        # each lands in errors, and the release happens in the end, whatever
        # happened (plan 3.2: the lock is never stranded).
        try:
            from ..gates.jobs import stop_products

            stop = stop_products
        except Exception as exc:  # jobs ships with the slice
            stop = None
            errors.append(f"the products could not be stopped: {exc}")
        for story in stories:
            if stop is not None:
                try:
                    stop(root, subject=story)
                except Exception as exc:
                    errors.append(f"the product of {story} could not be stopped: {exc}")
            try:
                changed = _close_story(root, cycle_id, story, outcome)
                closure_changed = closure_changed or changed
            except Exception as exc:
                errors.append(f"the closure of {story} could not be written: {exc}")
        try:
            from .commits import flush

            flushed = flush(root, f"checkpoint {workflow}", run=run_id)
            if not flushed["ok"]:
                errors.append(flushed.get("error") or f"the flush is {flushed.get('state', 'failed')}")
        except Exception as exc:
            errors.append(f"the flush failed: {exc}")
        push_gap = _push_gap(root)
    finally:
        finally_error = _release_path(root, run_id)
        if finally_error:
            errors.append(finally_error)
    if push_gap:
        errors.append(push_gap)

    progress = _had_steps(root, run_id) or closure_changed
    delivered = [_delivered(root, cycle_id, story) for story in stories]
    status = "failed" if outcome == "failed" else ("done" if stories and all(delivered) else "partial")
    output = {
        "ok": not errors,
        "error": "; ".join(errors),
        "status": status,
        "progress": progress,
        "decisions": [],
        "next": _route_next(root),  # next is computed after the release
    }
    return output, (0 if not errors else 1)


# ---------------------------------------------------------------- the body ---


def _close_story(root, cycle_id: str, story: str, outcome: str) -> bool:
    """Stage entries for the positions reached and not yet recorded (D9); on
    a delivered story the close fields and the ``deliver`` stage only. The
    ``closed`` field is the ISO stamp of this checkpoint, as the record
    family's ``closed`` is a stamp (records/closure.md)."""
    from .cycles import closure_read, closure_update
    from .story_unit import position
    from .store import now as now_stamp

    closure = closure_read(root, cycle_id, "story", story)
    where = position(root, cycle_id, story)
    delivery = (closure or {}).get("delivery") or {}
    merged = str(delivery.get("merged_sha", "") or "")
    stamp = now_stamp()

    def mutate(record: dict) -> None:
        stages = record.setdefault("stages", {})
        if merged:
            record["state"] = "closed"
            record["position"] = "delivered"
            record["closed"] = stamp
            deliver = stages.get("deliver")
            if not isinstance(deliver, dict) or deliver.get("state") != "done":
                stages["deliver"] = {
                    "state": "done",
                    "head": merged,
                    "merged_sha": merged,
                    "outcome": outcome or "done",
                }
            return
        reached = _POSITION_ORDER.index(where) if where in _POSITION_ORDER else 0
        head = _head_of(root, story)
        for pos in _POSITION_ORDER[1 : reached + 1]:
            stage = _STAGE_OF_POSITION[pos]
            entry = stages.get(stage)
            if isinstance(entry, dict) and entry.get("state") == "done":
                continue  # not yet recorded means recorded here only once
            stages[stage] = {"state": "done", "head": head}

    before = repr(sorted(((closure or {}).get("stages") or {}).items()))
    updated = closure_update(root, cycle_id, "story", story, mutate)
    return repr(sorted(updated.get("stages", {}).items())) != before


def _delivered(root, cycle_id: str, story: str) -> bool:
    from .cycles import closure_read

    try:
        closure = closure_read(root, cycle_id, "story", story)
        return bool(closure and (closure.get("delivery") or {}).get("merged_sha"))
    except Exception:  # an unreadable closure is not a delivered one
        return False


def _head_of(root, story: str) -> str:
    from .store import git as run_git
    from .worktrees import story_path

    try:
        return run_git(story_path(root, story), "rev-parse", "HEAD")
    except Exception:
        return ""


def _had_steps(root, run_id: str) -> bool:
    from .steps import run_has_steps

    try:
        return run_has_steps(root, run_id)
    except Exception:
        return False


def _push_gap(root) -> str:
    """D19: with ``state.push`` true and no push function shipped, the run
    ends all the same and the gap is named once the lock is released."""
    from ..config import ConfigError, load
    from .store import git as run_git

    try:
        config = load(root / ".factory" / "factory.toml")
        if not config.data["state"]["push"]:
            return ""
    except (ConfigError, KeyError, OSError):
        return ""
    try:
        remotes = run_git(root / ".factory", "remote").strip()
    except Exception:
        return ""
    if not remotes:
        return ""
    return "the state push is not built yet; the state branch has a remote, so run the push by hand or unset state.push"


def _release_path(root, run_id: str) -> str:
    """``run_set(agents)`` then ``lock_release`` (D18); an empty answer when
    the release is complete. A busy release is reported loudly and leaves the
    run looking live — never closed when it is not."""
    from .locks import lock_entries, lock_release
    from .runs import run_end, run_set
    from .steps import steps_list
    from .store import StoreBusy

    agents = 0
    try:
        agents = len(steps_list(root, run_id))
    except Exception:
        agents = 0
    try:
        run_set(root, run_id, agents=agents)
    except StoreBusy:
        pass  # the count is cosmetic next to the release that follows
    except Exception:
        pass
    try:
        lock_release(root, run=run_id)
    except StoreBusy as busy:
        kept = getattr(busy, "kept", [])
        return (
            f"LOUD: the lock release could not end the run ({len(kept)} kept) because a "
            "flush holds it; the run stays live and the lock entry stands — retry the "
            f"release when the flush finishes. released: "
            f"{len(getattr(busy, 'released', []))} entries"
        )
    except Exception as exc:
        return f"LOUD: the lock release failed: {exc}"
    entries = lock_entries(root)
    if not any(entry.get("run", "") == run_id for entry in entries):
        try:
            run_end(root, run_id)  # only when no lock entry names the run
        except Exception:
            pass
    return ""


def _route_next(root) -> dict:
    from .route import next_step

    try:
        return next_step(root)
    except Exception:
        return {"kind": "human", "command": "", "args": {"reason": "the route could not be computed"}}
