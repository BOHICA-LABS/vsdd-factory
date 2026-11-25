"""The convergence rule in Python (topics/convergence.md,
topics/prelude-converge.md; plan step-4 verdicts.md 3.1).

:data:`judge` is the prelude's ``judge()`` statement for statement, and
:data:`replay` is its ``converge()`` over recorded passes: where the script
would call ``c.review``/``c.fix``/``c.arbitrate``, the replay reads the pass's
own record. Pure, no I/O; ``vsdd.state.verdicts`` feeds it.

Three differences from the script, per the plan:

1. ``c.review(pass)`` is ``passes[pass-1].review`` and ``c.fix`` is
   ``passes[pass-1].fix``; ``c.arbitrate`` exists when ``arbiter`` is true and
   returns ``arbitration`` (null reads as no rulings).
2. Every recorded pass is walked. Where the script returns, the replay notes
   that outcome and, if a pass follows, goes on — with no claims after a pass
   that left nothing open, otherwise with the claims built from the open
   findings and the pass's fix (a missing fix reads as no resolutions).
3. If the last pass leaves findings open where the script goes on to a fix,
   the verdict is ``failed``, ``no_result`` with them in ``blocking``. With no
   pass at all it is ``blocked``, ``cap`` when ``used >= cap``, else ``failed``,
   ``no_result``.
"""

from __future__ import annotations

__all__ = []  # the facade lists only state-api names

HARD = ("critical", "high")

REACH = {
    "spec": ["unit", "integration", "system", "spec"],
    "plan": ["unit", "integration", "system", "spec"],
    "story": ["unit"],
    "wave": ["unit", "integration"],
    "release": ["unit", "integration", "system"],
}


def judge(findings, perimeter, waived) -> dict:
    """The prelude's ``judge()``: the four buckets in the findings' own order.

    ``KeyError`` for an unknown perimeter, as ``REACH[perimeter]`` raises.
    """
    v = {"blocking": [], "advisory": [], "deferred": [], "spec_gaps": []}
    for f in findings or []:
        # A holdout finding is built from scenario scores and has no file to
        # point at, so it needs no location.
        placed = f.get("category") == "holdout" or bool(
            f.get("location") and f["location"].get("path")
        )
        # grounded is false only on a finding whose citation failed the
        # grounding check twice; an absent mark leaves the judgement to the
        # other conditions.
        hard = (
            f.get("severity") in HARD
            and (f.get("category") == "holdout" or bool(f.get("spec_ref") or f.get("repro")))
            and placed
            and f.get("grounded") is not False
        )
        if f.get("key") in (waived or []):
            v["advisory"].append(f)
        elif f.get("scope") in REACH[perimeter]:
            (v["blocking"] if hard else v["advisory"]).append(f)
        elif f.get("scope") == "spec":
            (v["spec_gaps"] if hard else v["advisory"]).append(f)
        else:
            v["deferred"].append(f)
    return v


def _claims_for(open_, fix, disputed, notes):
    """The claims the script builds after a fix: structured data only, no
    title, no quote, none of the fixer's text; a holdout claim is the key
    alone, and a location member is claimed only where it exists (the
    script's undefined members vanish in JSON)."""
    resolutions = (fix or {}).get("resolutions") or []
    how: dict = {}
    for x in resolutions:  # the last entry for a key wins (decision D6)
        how[x.get("key")] = x
    claims = []
    for f in open_:
        h = how.get(f.get("key")) or {}
        if h.get("resolution") == "disputed":
            disputed.add(f.get("key"))
            notes[f.get("key")] = h.get("note") or ""
        claim = {"key": f.get("key")}
        if f.get("category") != "holdout" and isinstance(f.get("location"), dict):
            for member in ("path", "line_start", "line_end"):
                if f["location"].get(member) is not None:
                    claim[member] = f["location"][member]
        claim["resolution"] = h.get("resolution") or "fixed"
        claim["commit"] = h.get("commit") or ""
        claims.append(claim)
    return claims


def replay(loop, trace=None) -> dict:
    """The prelude's ``converge()`` over the recorded passes of one loop.

    ``loop`` carries ``perimeter``, ``subject``, ``cap``, ``used``,
    ``waived``, ``arbiter``, ``arbitration`` and ``passes``; the verdict is
    the state after the last pass. With ``trace`` a list, one entry per pass
    is appended — ``{pass, claims, gaps}``, plus ``fix`` when the fixer is
    asked and ``arbitrate`` when the arbiter is.
    """
    perimeter = loop["perimeter"]
    subject = loop.get("subject", "")
    cap = loop.get("cap", 0)
    used = loop.get("used") or 0
    waived = list(loop.get("waived") or [])
    arbiter = loop.get("arbiter") is True
    arbitration = loop.get("arbitration") or None
    passes = loop.get("passes") or []

    raised: dict = {}
    dismissed: list = []
    disputed: set = set()
    notes: dict = {}
    claims: list = []
    open_: list = []
    advisory: list = []
    deferred: list = []
    gaps: list = []
    head = ""
    pass_no = 0
    extra = 0
    arbitrated = False
    outcome = None

    def verdict(outcome_, reason_):
        return {
            "perimeter": perimeter,
            "subject": subject,
            "head": head,
            "passes": pass_no,
            "rounds_used": used + pass_no,
            "outcome": outcome_,
            "reason": reason_,
            "blocking": open_,
            "advisory": advisory,
            "deferred": deferred,
            "dismissed": dismissed,
            "coverage_missing": gaps,
        }

    # An item that no reviewer marked checked is itself a finding, so the
    # fixer is called and the loop can end.
    def uncovered(item):
        return {
            "key": f"coverage:{subject}:{item}",
            "title": f"Not accounted for: {item}",
            "severity": "high",
            "category": "coverage",
            "scope": REACH[perimeter][0],
            "owner": "test-writer",
            "location": {"path": ""},
            "spec_ref": item,
            "repro": "",
            "observed": "No reviewer marked this item checked.",
            "expected": "A test or another evidence route covers it.",
        }

    for holder in passes:
        pass_no += 1
        outcome = None
        entry = None
        if trace is not None:
            entry = {"pass": pass_no, "claims": list(claims), "gaps": list(gaps)}
            trace.append(entry)
        review = holder.get("review")
        if review is not None and review.get("blocked"):
            outcome = ("blocked", "reviewer_blocked")
        elif not review or not review.get("ok"):
            outcome = ("failed", "no_result")
        else:
            head = review.get("head", "")
            j = judge(review.get("findings"), perimeter, waived)
            for f in j["blocking"]:
                raised[f["key"]] = f
            checked = {k["key"]: k.get("outcome") for k in review.get("claim_checks") or []}
            for k in claims:
                if checked.get(k["key"]) == "withdrawn":
                    dismissed.append(
                        {"key": k["key"], "by": "reviewer", "reason": "Withdrawn on re-review."}
                    )
            still = [
                raised.get(k["key"])
                for k in claims
                if checked.get(k["key"]) not in ("resolved", "withdrawn")
            ]
            gaps = [
                item
                for item in review.get("coverage_missing") or []
                if f"coverage:{subject}:{item}" not in waived
            ]
            cov = [uncovered(item) for item in gaps]
            for f in cov:
                raised[f["key"]] = f
            open_ = j["blocking"] + [
                f
                for f in still
                if f and f.get("category") != "coverage"
                and not any(b.get("key") == f.get("key") for b in j["blocking"])
            ] + cov
            advisory = j["advisory"]
            deferred = deferred + j["deferred"]
            if j["spec_gaps"]:
                open_ = j["spec_gaps"]
                outcome = ("blocked", "spec_gap")
            elif not open_:
                outcome = ("converged", "clean")
        if outcome is None:
            if used + pass_no >= cap + extra:
                if extra or not arbiter:
                    outcome = ("blocked", "cap")  # the script breaks here
                elif not arbitrated:
                    # Applied once, at the first pass at the cap with findings open.
                    arbitrated = True
                    if trace is not None and entry is not None:
                        entry["arbitrate"] = {
                            "keys": [f.get("key") for f in open_],
                            "notes": [[k, n] for k, n in notes.items()],
                        }
                    ruled = {x["key"]: x for x in (arbitration or {}).get("rulings") or []}
                    upheld = []
                    for f in open_:
                        x = ruled.get(f.get("key"))
                        if x and x.get("ruling") == "spec_gap":
                            open_ = [f]
                            outcome = ("blocked", "spec_gap")
                            break
                        # Only a finding the builder disputed can be dismissed,
                        # and never a critical or security finding.
                        if (
                            x
                            and x.get("ruling") == "not_defect"
                            and f.get("key") in disputed
                            and f.get("severity") != "critical"
                            and f.get("category") != "security"
                        ):
                            dismissed.append(
                                {
                                    "key": f.get("key"),
                                    "by": "arbiter",
                                    "reason": x.get("reason") or "",
                                }
                            )
                            waived.append(f.get("key"))
                        else:
                            upheld.append(f)
                    if outcome is None:
                        open_ = upheld
                        if not open_:
                            outcome = ("converged", "clean")
                        else:
                            extra = 1
                else:
                    # The arbitration is spent and findings are still open:
                    # the loop is capped, however many passes follow
                    # (S4R-verdicts-12) — replay never ends failed/no_result
                    # where the script answered blocked, cap.
                    outcome = ("blocked", "cap")
        if outcome is None:
            if trace is not None and entry is not None:
                entry["fix"] = [f.get("key") for f in open_]
            fix = holder.get("fix")
            if not fix or not fix.get("ok"):
                outcome = ("failed", "no_result")
            else:
                claims = _claims_for(open_, fix, disputed, notes)
        if outcome is not None and pass_no < len(passes):
            # The script returned here, but the records go on: the next pass
            # is reviewed with no claims after a pass that left nothing open,
            # else with the claims from the open findings and this pass's fix
            # (a missing fix reads as no resolutions).
            if outcome == ("converged", "clean"):
                claims = []
            else:
                claims = _claims_for(open_, holder.get("fix"), disputed, notes)

    if outcome is None:
        if not passes:
            outcome = ("blocked", "cap") if used >= cap else ("failed", "no_result")
        else:
            # The fix failed, or the loop was cut off after its last fix.
            outcome = ("failed", "no_result")
    return verdict(*outcome)
