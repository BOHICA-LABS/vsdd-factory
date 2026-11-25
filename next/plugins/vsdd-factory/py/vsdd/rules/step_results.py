"""The step-results rule (rules/step-results.md; plan rules.md 3.4).

Delivery-order gate H7: the result a workflow step returns is checked — every
passing gate entry against the evidence store, every review finding against
the reviewed tree — and then stored, before the call completes. The rule is
locked: a step result that is not on disk breaks resume, and an ungrounded
finding must go back to its author once, marked, before it is stored.

The one grounding decision of the project lives here and in
:mod:`vsdd.grounding`: an agent's own ``grounded`` key is never stored, and a
``Rewrite`` travels with the stored result whenever it differs from the input
(decision D13), so the script judges what was stored.
"""

from __future__ import annotations

import copy
import hashlib
import json

__all__: list[str] = []

_REJECTED = "step.rejected"
_RECORDED = "step.recorded"

_UNGROUNDED_FIX = (
    "Correct the location or the quote of each finding and return again; the "
    "next return is stored, the still-ungrounded findings marked advisory "
    "(grounded: false)."
)


def check(ctx, cfg):
    """Store one step result, or refuse it loudly."""
    result = ctx.tool_input
    step = result.get("step") if isinstance(result, dict) else None
    if not isinstance(step, dict):
        return None  # no step identity: the call passes unstored (decision D10)
    run = step.get("run")
    if not isinstance(run, str) or not run:
        return None

    project = ctx.project
    blocks = []

    # Walled findings refuse out loud, every time (decision D15): the
    # scenario-reviewer's findings quote hidden scenarios.
    findings = _findings(result)
    if (ctx.agent_type == "vsdd-factory:scenario-reviewer" and findings) or _holdout(findings):
        blocks.append(
            _block(
                "walled_not_built",
                "the result carries findings of the holdout wall; the holdout "
                "evaluation file is not built yet, and no walled text is stored "
                "before the wall exists.",
            )
        )

    blocks.extend(_gate_blocks(project, result))

    failing = _grounding(project, result, step, blocks)
    marker = _marker_path(ctx, step)
    if failing:
        if marker.exists():
            pass  # the second return: the findings are stored marked
        else:
            fault = _write_marker(marker, step)
            if fault is not None:
                ctx.log(_REJECTED, "warn", code="marker_unwritable", **_identity(step))
                return [fault]
            ctx.log(_REJECTED, "warn", code="findings_ungrounded", **_identity(step))
            # Beside the other blocks, never instead of them: the agent gets
            # the whole picture in one return, and the marker stands so the
            # next return is stored (plan 3.4; S4R-rules-9).
            blocks.append(
                _block("findings_ungrounded", _ungrounded_lines(failing), _UNGROUNDED_FIX)
            )

    if blocks:
        ctx.log(_REJECTED, "warn", code=blocks[0].code, **_identity(step))
        return blocks
    return _store(ctx, project, result, step, failing, marker)


# --- the store ------------------------------------------------------------------


def _store(ctx, project, result, step, failing, marker):
    """Store the marked result; a refusal blocks and stores nothing."""
    from ..hooks import gitfs
    from ..state import steps as step_store, worktrees
    from ..state.store import StoreError, StoreRefused

    adjusted = _marked(result, failing)
    try:
        tree = worktrees.unit_tree(project, step.get("workflow", ""), step.get("subject", ""))
        head = gitfs.head_sha(tree) or ""
    except Exception:
        head = ""
    try:
        stored = step_store.store_step(
            project, adjusted, ctx.agent_type, head, agent_id=ctx.agent_id
        )
    except StoreRefused as exc:
        ctx.log(_REJECTED, "warn", code=exc.code, **_identity(step))
        return _block(
            exc.code,
            f"the step store refused the result ({exc}); the step record is "
            "not written, so the step counts for nothing.",
            "Answer again with a result the store takes: the run must be live "
            "and the identity the prompt gave.",
        )
    except StoreError as exc:
        ctx.log(_REJECTED, "warn", code="step_store_failed", **_identity(step))
        return _block(
            "step_store_failed",
            f"the step store failed ({exc}); nothing is stored.",
            "Answer again; if it repeats, report the fault.",
        )
    _remove_marker(marker)
    ctx.log(_RECORDED, "info", **_identity(step))
    if adjusted != result:
        from ..hooks.model import Rewrite

        # The marked input, not the store's record: the record carries the
        # runner's own fields (agent_type, worktree_head, agent_id) beside
        # what the agent sent (plan 3.4; S4R-rules-24).
        return Rewrite(adjusted, "the step result with the grounding mark")
    return None


def _marked(result, failing):
    """The input as it is stored: every agent-supplied ``grounded`` key
    removed, ``grounded: false`` set on each failing finding — by its
    position in the findings list, not its key, which a finding may lack or
    share (decision D13, S4R-rules-8)."""
    adjusted = copy.deepcopy(result)
    positions = {position for position, _key, _why in failing}
    for position, finding in enumerate(_finding_nodes(adjusted)):
        if not isinstance(finding, dict):
            continue
        finding.pop("grounded", None)
        if position in positions:
            finding["grounded"] = False
    return adjusted


# --- the checks -------------------------------------------------------------------


def _gate_blocks(project, result):
    """Every gate entry with ``passed: true`` matches its evidence record, found
    by subject, gate and commit — never by the path the entry carries
    (decision D11). A malformed entry and a store fault are a mismatch."""
    from ..state import gate_evidence
    from ..state.store import StoreError

    blocks = []
    gates = result.get("gates")
    if not isinstance(gates, list):
        return blocks
    for entry in gates:
        if not isinstance(entry, dict) or entry.get("passed") is not True:
            continue
        gate = entry.get("gate")
        subject = entry.get("subject")
        sha = entry.get("sha")
        if not all(isinstance(value, str) and value for value in (gate, subject, sha)):
            blocks.append(_mismatch(gate, subject, sha))
            continue
        try:
            record = gate_evidence.evidence_read(project, subject, gate, sha)
        except StoreError:
            record = None
        if record is None:
            blocks.append(_mismatch(gate, subject, sha))
            continue
        matches = (
            record.get("gate", "") == gate
            and record.get("subject", "") == subject
            and record.get("sha", "") == sha
            and record.get("passed") is True
        )
        if not matches:
            blocks.append(_mismatch(gate, subject, sha))
    return blocks


def _mismatch(gate, subject, sha):
    shown = f"{gate or '?'}/{str(sha or '?')[:12]}"
    return _block(
        "gate_result_mismatch",
        f"the gate result {shown} for {subject or '?'} has no matching record "
        "in the evidence store: a gate result is stored by the tool that ran "
        "it, and this one is not there.",
        "Run the step's closing command (vsdd gate ...) and return its output "
        "unchanged.",
    )


def _grounding(project, result, step, blocks):
    """Ground every finding with a place to check; ``tree_unknown`` when the
    reviewed tree cannot be found and a finding needed it. Returns the failing
    findings as ``(position, key, reason)`` triples, the position naming the
    node in the findings list — the mark is by position, and the key only
    dresses the block text (S4R-rules-8)."""
    from .. import grounding
    from ..state import worktrees
    from pathlib import Path

    nodes = _finding_nodes(result)
    needs_tree = [
        node
        for node in nodes
        if isinstance(node, dict) and node.get("category") != "holdout" and _path_of(node)
    ]
    failing: list[tuple[int, str, str]] = []
    if not needs_tree:
        return failing
    try:
        tree = worktrees.unit_tree(project, step.get("workflow", ""), step.get("subject", ""))
    except Exception as exc:
        blocks.append(
            _block(
                "tree_unknown",
                "the reviewed tree of the step could not be found (" + str(exc) + "); "
                "the findings cannot be grounded.",
                "Admit the story again so its worktree exists, and answer again.",
            )
        )
        return failing
    for position, node in enumerate(nodes):
        if not isinstance(node, dict):
            continue
        if node.get("category") == "holdout" or not _path_of(node):
            continue
        why = grounding.ground(node, tree=tree, project_root=Path(project.root))
        if why:
            failing.append((position, str(node.get("key", "?")), why))
    return failing


def _ungrounded_lines(failing) -> str:
    return "\n".join(f"finding {key}: {why}" for _position, key, why in failing)


# --- the once-per-step marker (decision D12) ----------------------------------------


def _marker_path(ctx, step):
    directory = ctx.project.hooks_dir / "steps"
    identity = "{run}/{workflow}/{subject}/{name}.{attempt}".format(
        run=step.get("run", ""),
        workflow=step.get("workflow", ""),
        subject=step.get("subject", ""),
        name=step.get("name", ""),
        attempt=step.get("attempt", ""),
    )
    return directory / hashlib.sha256(identity.encode("utf-8")).hexdigest()[:32]


def _write_marker(marker, step):
    """Write the marker; a fault answers the block, success answers ``None``."""
    try:
        marker.parent.mkdir(parents=True, exist_ok=True)
        marker.write_text(
            json.dumps({"identity": _identity(step)}, sort_keys=True) + "\n",
            encoding="utf-8",
        )
    except OSError:
        return _block(
            "marker_unwritable",
            f"the once-per-step marker at {marker} could not be written; the "
            "result cannot be stored safely.",
            "Make .factory/local writable for the session's user.",
        )
    return None


def _remove_marker(marker):
    try:
        marker.unlink()
    except OSError:
        pass  # a stale marker is H9's to prune; the finding marks once more


# --- finding helpers -----------------------------------------------------------------


def _findings(result):
    """The result's finding objects: ``data.findings``, or a top-level list."""
    data = result.get("data")
    if isinstance(data, dict) and isinstance(data.get("findings"), list):
        return [finding for finding in data["findings"] if isinstance(finding, dict)]
    if isinstance(result.get("findings"), list):
        return [finding for finding in result["findings"] if isinstance(finding, dict)]
    return []


def _finding_nodes(result):
    """Every node the stored (marked) copy's findings list holds."""
    data = result.get("data")
    if isinstance(data, dict) and isinstance(data.get("findings"), list):
        return data["findings"]
    if isinstance(result.get("findings"), list):
        return result["findings"]
    return []


def _holdout(findings):
    """Whether a finding's location lies under the holdout wall. The one
    predicate is the store's (S4R-rules-25): the rule must not answer a
    question the store settles differently."""
    from ..state.steps import _under_holdout

    for finding in findings:
        path = _path_of(finding)
        if path and _under_holdout(path):
            return True
    return False


def _path_of(finding):
    location = finding.get("location")
    if not isinstance(location, dict):
        return None
    path = location.get("path")
    return path if isinstance(path, str) and path else None


def _identity(step):
    return {
        "run": step.get("run", ""),
        "workflow": step.get("workflow", ""),
        "subject": step.get("subject", ""),
        "name": step.get("name", ""),
        "attempt": step.get("attempt", ""),
    }


def _block(code: str, reason: str, fix: str = ""):
    from ..hooks.model import Block

    return Block(code, reason, fix)
