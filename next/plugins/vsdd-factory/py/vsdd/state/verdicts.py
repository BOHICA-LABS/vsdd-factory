"""The verdict rebuilt from a story's stored passes (topics/verdicts.md,
topics/evidence-lookup.md; plan step-4 verdicts.md, package G3).

``vsdd.state.verdict`` is the one flat name (state-api.md). Everything else
here is reached through the module: the convergence rule lives in
:mod:`vsdd.state.convergence`, and this module rebuilds the loop inputs from
the step records (``rebuild``) and replays them.

Nothing here grounds a finding (PL4-1): the stored ``grounded`` mark is
trusted (decision D5), because grounding ran once, when the result was
stored, and later fixes would unground the findings of early passes (VSA-07).
A step record that does not parse raises records' ``StoreCorrupt`` (D14):
skipping it could turn an open loop into a converged one.
"""

from __future__ import annotations

__all__ = ["verdict"]

_PLUGIN_PREFIX = "vsdd-factory:"

#: The review methods of the story perimeter: method to (owner, whether its
#: ``coverage`` accounts for the review items). Held to the roster by
#: ``test_review_methods_table``; no shipped table says it twice (decision D2).
REVIEW_METHODS = {
    "review-story-diff": ("adversary", True),
    "security-review": ("security-reviewer", False),
}

#: The arbiter's method and owner; a stored type owns it as
#: ``vsdd-factory:<owner>``.
ARBITER = ("arbitrate", "architect")

#: The fix method whose resolutions join first (decision D6).
FIX_FIRST = "test-fixes"

_EMPTY_VERDICT = {
    "perimeter": "story",
    "subject": "",
    "head": "",
    "passes": 0,
    "rounds_used": 0,
    "outcome": "failed",
    "reason": "no_result",
    "blocking": [],
    "advisory": [],
    "deferred": [],
    "dismissed": [],
    "coverage_missing": [],
}


def changed_files(project, since: str, head: str) -> list[str] | None:
    """The files changed between two commits, sorted — ``git diff --name-only
    --no-renames`` at the project root. ``None`` when git cannot say.

    The one definition of "the files changed" (VSA-03, PL4-7): the green
    gate's ``--since`` result and delivery call this and write none of their
    own. ``--no-renames`` so two callers cannot differ (decision D7): a
    renamed file is a deletion and an addition, and both names count.
    """
    from .gitrun import GitError, git
    from .store import root_of

    since, head = str(since or ""), str(head or "")
    if not since or not head:
        return None
    try:
        out = git(
            root_of(project),
            "-c",
            "core.quotePath=false",
            "diff",
            "--name-only",
            "-z",
            "--no-renames",
            since,
            head,
        )
    except GitError:
        return None
    return sorted(line for line in out.split("\0") if line)


# ------------------------------------------------------------- the rebuild ---


def _valid(data, schema: str) -> bool:
    from ..schema import is_valid
    from .store import _registry, schema_of

    return isinstance(data, dict) and is_valid(data, schema_of(schema), _registry())


def _owner_of(method: str) -> str | None:
    """The stored ``agent_type`` that owns ``method``, or ``None`` for a
    method that is neither a review method nor the arbiter's."""
    if method in REVIEW_METHODS:
        return _PLUGIN_PREFIX + REVIEW_METHODS[method][0]
    if method == ARBITER[0]:
        return _PLUGIN_PREFIX + ARBITER[1]
    return None


def _classify(records, subject: str) -> dict:
    """Rules 2 to 5 of the plan's rebuild: which records count, and why the
    rest do not. The winner of one ``(run, loop, pass, method)`` is its
    highest attempt; the loser is skipped with ``attempt``."""
    reviews: dict = {}
    arbitrations: dict = {}
    fixes: dict = {}
    skipped: list = []
    walled: list = []
    first_index_of_run: dict = {}
    order = 0
    for index, record in enumerate(records):
        step = record.get("step") or {}
        if step.get("workflow") != "story" or step.get("subject") != subject:
            continue
        run = str(step.get("run", "") or "")
        first_index_of_run.setdefault(run, order)
        order += 1
        _walled_from(record, walled)
        pass_no = step.get("pass")
        if not isinstance(pass_no, int) or isinstance(pass_no, bool) or pass_no < 1:
            continue  # without a pass nothing counts it, and nothing lists it
        loop_no = step.get("loop") or 1
        attempt = step.get("attempt", 0)
        method = str(step.get("method", "") or "")
        status = record.get("status", "")
        step_key = f"{step.get('name', '')}.{attempt}"

        owner = _owner_of(method)
        if owner is not None and record.get("agent_type", "") != owner:
            skipped.append({"index": index, "run": run, "step": step_key, "why": "agent_type"})
            continue
        if method in REVIEW_METHODS:
            if status == "blocked":
                state = "blocked"
            elif status == "done":
                state = "ok" if _valid(record.get("data"), "review-data") else "invalid"
            else:
                continue  # any other status is ignored
            key = (run, loop_no, pass_no, method)
            candidate = {
                "attempt": attempt,
                "index": index,
                "step": step_key,
                "state": state,
                "data": record.get("data") or {},
                "method": method,
            }
            prior = reviews.get(key)
            if prior is None or attempt > prior["attempt"]:
                if prior is not None:
                    skipped.append(
                        {"index": prior["index"], "run": run, "step": prior["step"], "why": "attempt"}
                    )
                reviews[key] = candidate
            else:
                skipped.append({"index": index, "run": run, "step": step_key, "why": "attempt"})
            continue
        if method == ARBITER[0]:
            if status != "done":
                continue
            if not _valid(record.get("data"), "ruling-data"):
                skipped.append({"index": index, "run": run, "step": step_key, "why": "data"})
                continue
            key = (run, loop_no)
            prior = arbitrations.get(key)
            if prior is None or attempt > prior["attempt"]:
                if prior is not None:
                    skipped.append(
                        {"index": prior["index"], "run": run, "step": prior["step"], "why": "attempt"}
                    )
                arbitrations[key] = {
                    "attempt": attempt,
                    "index": index,
                    "step": step_key,
                    "data": record.get("data") or {},
                }
            else:
                skipped.append({"index": index, "run": run, "step": step_key, "why": "attempt"})
            continue
        if status != "done":
            continue
        if not _valid(record.get("data"), "fix-data"):
            skipped.append({"index": index, "run": run, "step": step_key, "why": "data"})
            continue
        head = record.get("head", "")
        if not any(
            isinstance(gate, dict) and gate.get("passed") is True and gate.get("sha", "") == head
            for gate in record.get("gates") or []
        ):
            skipped.append({"index": index, "run": run, "step": step_key, "why": "gate"})
            continue
        fixes.setdefault((run, loop_no, pass_no), []).append(
            {"method": method, "data": record.get("data") or {}}
        )
    skipped.sort(key=lambda entry: entry["index"])
    return {
        "reviews": reviews,
        "arbitrations": arbitrations,
        "fixes": fixes,
        "skipped": skipped,
        "walled": walled,
        "first_index_of_run": first_index_of_run,
    }


def _walled_from(record: dict, walled: list) -> None:
    """A walled finding is one returned by ``vsdd-factory:scenario-reviewer``
    or located under ``.factory/holdout/`` (topic walled-findings)."""
    data = record.get("data")
    if not isinstance(data, dict) or record.get("status", "") != "done":
        return
    findings = data.get("findings")
    if not isinstance(findings, list):
        return
    from .steps import _under_holdout

    for finding in findings:
        if not isinstance(finding, dict) or not finding.get("key"):
            continue
        location = finding.get("location") or {}
        path = location.get("path", "") if isinstance(location, dict) else ""
        if record.get("agent_type", "") == _PLUGIN_PREFIX + "scenario-reviewer" or (
            isinstance(path, str) and _under_holdout(path)
        ):
            if finding["key"] not in walled:
                walled.append(finding["key"])


def _coverage_missing(items, winners) -> list:
    """Rule 7: the loop's items that no record of an accounting method marks
    ``checked`` or ``not_checkable`` (S5P-N9: only the lens that owns the
    items accounts for them)."""
    accounted: set = set()
    for winner in winners:
        if not REVIEW_METHODS[winner["method"]][1] or winner["state"] != "ok":
            continue
        for entry in winner["data"].get("coverage") or []:
            if entry.get("state") in ("checked", "not_checkable"):
                accounted.add(entry.get("item"))
    return [item for item in items if item not in accounted]


def _first_pass_head(groups) -> str | None:
    """The head of the loop's first pass: the one ``reviewed_sha`` its valid
    winners agree on; ``None`` when that first pass is not ok."""
    for pass_no in sorted({key[2] for key in groups}):
        winners = [groups[key] for key in groups if key[2] == pass_no]
        blocked = any(w["state"] == "blocked" for w in winners)
        invalid = any(w["state"] == "invalid" for w in winners)
        valid = [w for w in winners if w["state"] == "ok"]
        heads = {str(w["data"].get("reviewed_sha", "")) for w in valid}
        if blocked or invalid or not valid or len(heads) != 1:
            return None
        return next(iter(heads))
    return None


def rebuild(records, *, subject: str, cap: int, waived, acs, changed) -> dict:
    """The pure core: the same result :func:`rebuilt` returns, from step
    records in stored order. ``changed(since, head)`` returns a list or
    ``None`` (plan step-4 verdicts.md 3.3, rules 1 to 9)."""
    classified = _classify(records, subject)
    return _rebuild(classified, subject=subject, cap=cap, waived=waived, acs=acs, changed=changed)


def _rebuild(classified, *, subject, cap, waived, acs, changed) -> dict:
    from .convergence import replay

    empty = dict(_EMPTY_VERDICT, subject=subject)
    skipped = [
        {"run": entry["run"], "step": entry["step"], "why": entry["why"]}
        for entry in classified["skipped"]
    ]
    loop_map: dict = {}
    for key in classified["reviews"]:
        loop_map.setdefault((key[0], key[1]), {})[key] = classified["reviews"][key]
    if not loop_map:
        return {
            "verdict": empty,
            "loops": [],
            "skipped": skipped,
            "chain": False,
            "passes_used": 0,
            "walled": classified["walled"],
        }
    first_of_run = classified["first_index_of_run"]
    ordered_loops = sorted(loop_map, key=lambda k: (first_of_run.get(k[0], 0), k[1]))
    loops = []
    previous_head = None
    verdict = empty
    for loop_key in ordered_loops:
        run, loop_no = loop_key
        groups = loop_map[loop_key]
        first_pass_head = _first_pass_head(groups)
        if loop_no == 1:
            items: list | None = list(acs)
        else:
            # A later loop accounts for the files changed since the verdict of
            # the loop before it, at the head its own first pass reviewed;
            # not computable makes its passes not ok (rule 8, VSA-03).
            if previous_head is None or first_pass_head is None:
                items = None
            else:
                items = changed(previous_head, first_pass_head)
        used = 0
        if loop_no == 1:
            # Rule 9: the loop-1 passes of earlier runs only. The stored
            # closure count is not a floor here (S4R-verdicts-C1-1): it
            # cannot tell an earlier run's passes from this run's own, so a
            # checkpoint would move the arbiter onto another pass and make
            # every converged story read not reviewed after it. What the run
            # was given reaches the script through the survey (S3/W2), and a
            # clone with no or short records follows the closure's stored
            # verdict through ``covering`` (decision D8).
            used = len(
                {
                    (pair_run, pass_no)
                    for (pair_run, pair_loop, pass_no, _method) in classified["reviews"]
                    if pair_loop == 1 and first_of_run.get(pair_run, 0) < first_of_run.get(run, 0)
                }
            )
        passes = []
        for pass_no in sorted({key[2] for key in groups}):
            winners = [groups[key] for key in groups if key[2] == pass_no]
            winners.sort(key=lambda w: list(REVIEW_METHODS).index(w["method"]))
            blocked = any(w["state"] == "blocked" for w in winners)
            invalid = any(w["state"] == "invalid" for w in winners)
            valid = [w for w in winners if w["state"] == "ok"]
            heads = {str(w["data"].get("reviewed_sha", "")) for w in valid}
            ok = (
                bool(valid)
                and not blocked
                and not invalid
                and len(heads) == 1
                and items is not None
            )
            if blocked:
                review = {"ok": False, "blocked": True}
            elif ok:
                findings: list = []
                checks: list = []
                for method in REVIEW_METHODS:
                    for winner in valid:
                        if winner["method"] == method:
                            findings.extend(winner["data"].get("findings") or [])
                            checks.extend(winner["data"].get("claim_checks") or [])
                review = {
                    "ok": True,
                    "head": next(iter(heads)),
                    "findings": findings,
                    "claim_checks": checks,
                    "coverage_missing": _coverage_missing(items, winners),
                }
            else:
                review = {"ok": False}
            entries = classified["fixes"].get((run, loop_no, pass_no), [])
            resolutions: list = []
            for entry in sorted(entries, key=lambda e: (0 if e["method"] == FIX_FIRST else 1,)):
                resolutions.extend(entry["data"].get("resolutions") or [])
            passes.append(
                {
                    "review": review,
                    "fix": {"ok": True, "resolutions": resolutions} if entries else None,
                }
            )
        ruling = classified["arbitrations"].get(loop_key)
        input_ = {
            "perimeter": "story",
            "subject": subject,
            "cap": cap if loop_no == 1 else 2,
            "used": used if loop_no == 1 else 0,
            "waived": list(waived),
            "arbiter": True,
            "passes": passes,
            "arbitration": (ruling or {}).get("data"),
        }
        verdict = replay(input_)
        loops.append({"run": run, "loop": loop_no, "input": input_, "verdict": verdict})
        previous_head = verdict.get("head", "") or None
    return {
        "verdict": verdict,
        "loops": loops,
        "skipped": skipped,
        "chain": _chain(loops),
        "passes_used": len(
            {
                (run, pass_no)
                for (run, loop_no, pass_no, _method) in classified["reviews"]
                if loop_no == 1
            }
        ),
        "walled": classified["walled"],
    }


def _chain(loops) -> bool:
    """Rule 9: true when, from the last loop back to the nearest loop 1, every
    verdict is converged."""
    for entry in reversed(loops):
        if entry["verdict"].get("outcome") != "converged":
            return False
        if entry["loop"] == 1:
            return True
    return False


# ------------------------------------------------------- the story's tools ---


def _story_tree(root, subject: str):
    """``.worktrees/<subject>/`` when it exists, else ``None``."""
    from . import worktrees

    try:
        tree = worktrees.story_path(root, subject)
    except worktrees.WorktreeError:
        return None
    return tree if (tree / ".git").exists() else None


def _patch_id(tree, base: str, head: str) -> str:
    from .gitrun import GitError
    from .store import StoreRefused

    if tree is None or not base or not head:
        return ""
    from .gate_evidence import patch_id

    try:
        return patch_id(tree, base, head)
    except (GitError, StoreRefused):
        return ""


def waivers(project, subject: str) -> list[str]:
    """The settled waivers for the story: from every approved ``finding``
    decision with the id ``finding.<subject>``, ``waive`` gives each word of
    ``values['key']`` and ``waive-coverage`` gives ``coverage:<subject>:<item>``
    for each word of ``values['item']`` (PL4-5: this one builder of
    ``waived``)."""
    from .decisions import decisions_list

    wanted = f"finding.{subject}"
    out: list[str] = []
    for _qualified, record in decisions_list(project, kind="finding"):
        if record.get("id") != wanted or record.get("status") != "approved":
            continue
        choice = record.get("choice", "")
        values = record.get("values") or {}
        field = "key" if choice == "waive" else "item"
        for word in str(values.get(field, "")).split():
            item = word if choice == "waive" else f"coverage:{subject}:{word}"
            if item and item not in out:
                out.append(item)
    return out


def passes_used(project, subject: str) -> int:
    """The number of distinct ``(run, pass)`` pairs of loop 1 that hold a
    reviewer record, over every run."""
    from . import steps

    records = steps.steps_list(project, workflow="story", subject=subject)
    classified = _classify(records, subject)
    return len(
        {
            (run, pass_no)
            for (run, loop_no, pass_no, _method) in classified["reviews"]
            if loop_no == 1
        }
    )


def _closure_rounds_used(project, subject: str) -> int:
    """The stored closure verdict's ``rounds_used`` (0 without one)."""
    from .cycles import closure_find

    closure = closure_find(project, "story", subject)
    if closure is None:
        return 0
    value = (closure.get("verdict") or {}).get("rounds_used")
    if isinstance(value, int) and not isinstance(value, bool) and value > 0:
        return value
    return 0


def rounds_used(project, subject: str) -> int:
    """The larger of :func:`passes_used` and the stored closure verdict's
    ``rounds_used`` (0 without one), so a clone with no run records never
    lowers it. The story unit's ``rounds_used``, which the script passes as
    ``used`` (decision D13)."""
    return max(passes_used(project, subject), _closure_rounds_used(project, subject))


def _config_cap(root) -> int:
    from ..config import load

    config = load(root / ".factory" / "factory.toml")
    data = getattr(config, "data", config)
    value = (data.get("convergence") or {}).get("passes", 3)
    if isinstance(value, int) and not isinstance(value, bool) and value >= 1:
        return value
    return 3


def _story_acs(root, subject: str) -> list[str]:
    """The story's acceptance criteria, read from the ``### AC-N`` headings
    outside code fences (decision D9). One reader with ``story_unit``'s, so
    the verdict's items and the unit's position cannot drift
    (``test_items_equal_unit_acs`` holds them together)."""
    from .artifacts import load_artifact
    from .story_unit import acs

    return acs(load_artifact(root, subject).body)


def _compute(project, subject: str) -> tuple[dict, dict | None]:
    """The verdict and its rebuild, or ``(empty, None)`` when no counted
    reviewer record exists in any run."""
    from . import steps
    from .store import root_of

    records = steps.steps_list(project, workflow="story", subject=subject)
    classified = _classify(records, subject)
    if not classified["reviews"]:
        return dict(_EMPTY_VERDICT, subject=subject), None
    root = root_of(project)
    rebuilt_ = _rebuild(
        classified,
        subject=subject,
        cap=_config_cap(root),
        waived=waivers(project, subject),
        acs=_story_acs(root, subject),
        changed=lambda since, head: changed_files(project, since, head),
    )
    return rebuilt_["verdict"], rebuilt_


def verdict(project, subject: str) -> dict:
    """The verdict of the story's last loop, valid against
    ``verdict.schema.json``; findings keep their stored ``grounded`` mark.

    With no counted reviewer record: ``passes`` 0, ``rounds_used`` 0,
    ``outcome`` ``failed``, ``reason`` ``no_result``, empty lists — never
    ``None``; "no pass is stored" is tested as ``summary(...) is None``
    (PL4-4). Raises records' ``StoreCorrupt`` for an unreadable step record
    and the artifact registry's ``ArtifactError`` when loops exist and the
    story file is gone. Any id is read as a story id."""
    computed, _rebuilt = _compute(project, subject)
    return computed


def rebuilt(project, subject: str) -> dict:
    """``{verdict, loops, skipped, chain, passes_used, walled}``: what
    :func:`verdict` computes, with the loops' inputs, the reasons a record was
    left out, and the walled findings' keys (decision D12)."""
    computed, rebuilt_ = _compute(project, subject)
    if rebuilt_ is not None:
        return rebuilt_
    classified = _classify(_records(project, subject), subject)
    return {
        "verdict": computed,
        "loops": [],
        "skipped": [
            {"run": entry["run"], "step": entry["step"], "why": entry["why"]}
            for entry in classified["skipped"]
        ],
        "chain": False,
        "passes_used": passes_used(project, subject),
        "walled": classified["walled"],
    }


def _records(project, subject: str) -> list:
    from . import steps

    return steps.steps_list(project, workflow="story", subject=subject)


def public_verdict(project, subject: str) -> dict:
    """:func:`verdict` with each walled finding replaced by its walled key,
    severity and the ``walled`` mark: all a packet or a closure may carry."""
    import hashlib

    computed, rebuilt_ = _compute(project, subject)
    if rebuilt_ is None:
        return computed
    walled = set(rebuilt_["walled"])

    def walled_key(key) -> str | None:
        if key in walled:
            digest = hashlib.sha256(str(key).encode("utf-8")).hexdigest()[:12]
            return f"walled:{digest}"
        return None

    def replace(finding):
        walled_as = walled_key(finding.get("key"))
        if walled_as is not None:
            return {
                "key": walled_as,
                "severity": finding.get("severity", ""),
                "walled": True,
            }
        return finding

    out = dict(computed)
    for field in ("blocking", "advisory", "deferred"):
        out[field] = [replace(finding) for finding in computed[field]]
    # A walled finding that was withdrawn or dismissed keeps its key in clear
    # here otherwise, and the arbiter's reason with it (S4R-verdicts-11); the
    # empty reason keeps the entry valid against verdict.schema.json.
    dismissed = []
    for entry in computed["dismissed"]:
        walled_as = walled_key(entry.get("key"))
        if walled_as is not None:
            dismissed.append({"key": walled_as, "by": entry.get("by", ""), "reason": ""})
        else:
            dismissed.append(entry)
    out["dismissed"] = dismissed
    return out


def summary(project, subject: str) -> dict | None:
    """The closure form of the computed verdict — what the checkpoint stores
    as the closure's ``verdict``. ``None`` exactly when the story has no
    counted reviewer record in any run; the closure then keeps what it has."""
    computed, rebuilt_ = _compute(project, subject)
    if rebuilt_ is None:
        return None
    return _closure_form(project, subject, computed)


def _closure_form(project, subject: str, computed: dict) -> dict:
    from .cycles import closure_find
    from .store import root_of

    root = root_of(project)
    tree = _story_tree(root, subject)
    closure = closure_find(project, "story", subject) or {}
    base = (closure.get("delivery") or {}).get("base", "")
    return {
        "perimeter": computed["perimeter"],
        "head": computed["head"],
        "patch_id": _patch_id(tree, base, str(computed["head"] or "")),
        "passes": computed["passes"],
        "rounds_used": rounds_used(project, subject),
        "outcome": computed["outcome"],
        "advisory": len(computed["advisory"]),
        "dismissed": len(computed["dismissed"]),
        "waived": len(waivers(project, subject)),
    }


def covering(project, subject: str, *, head: str | None = None) -> dict | None:
    """The review lookup of ``topics/evidence-lookup.md``: the computed
    verdict about ``head`` — by head, or by ``git patch-id`` of the diff from
    the closure's ``delivery.base`` — when it is converged and its chain holds;
    otherwise the closure's stored verdict when that is converged and covers.
    ``None`` without a worktree, a closure or a base (decision D8: the verdict
    about the commit asked about wins, computed before stored)."""
    from . import worktrees
    from .cycles import closure_find
    from .store import root_of

    root = root_of(project)
    tree = _story_tree(root, subject)
    if tree is None:
        return None
    closure = closure_find(project, "story", subject)
    if closure is None:
        return None
    base = (closure.get("delivery") or {}).get("base", "")
    if not base:
        return None
    head = head if head is not None else worktrees.head(tree)
    computed, rebuilt_ = _compute(project, subject)
    if _covers(tree, base, str(computed.get("head") or ""), head):
        if computed["outcome"] == "converged" and rebuilt_["chain"]:
            return _closure_form(project, subject, computed)
        return None
    stored = closure.get("verdict") or {}
    if stored.get("outcome") != "converged":
        return None
    # The stored verdict covers with its own head, by the same test as the
    # computed one: nothing writes ``delivery.head`` before a merge, and a
    # later commit recorded there must not serve a verdict about an earlier
    # one (S4R-verdicts-1).
    if _covers(tree, base, str(stored.get("head") or ""), head):
        return stored
    return None


def _covers(tree, base: str, verdict_head: str, head: str) -> bool:
    """The cover test: the same commit, or a non-empty patch id in common."""
    if verdict_head and verdict_head == head:
        return True
    if not verdict_head:
        return False
    ours = _patch_id(tree, base, verdict_head)
    return bool(ours) and ours == _patch_id(tree, base, head)
