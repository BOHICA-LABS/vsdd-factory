"""The verdicts unit: ``changed_files``, the rebuild from stored passes,
cover, waivers, and the rounds the story used (plan step-4 verdicts.md 7).

Projects come from ``story_ready`` and every step record is written through
``store_step`` — never hand-written record JSON (step-4 ORDER.md rule 5); the
walled-finding test goes through the pure ``rebuild`` because records refuses
to store a walled finding.
"""

from __future__ import annotations

import hashlib
import itertools
import subprocess
import tomllib
import unicodedata
from pathlib import Path

import pytest

from vsdd.state import cycles, runs, store_step, verdicts
from vsdd.state.store import SCHEMA_DIR, StoreCorrupt, StoreRefused

from . import story_ready

SUBJECT = "S-9.01"
REVIEW = "review-story-diff"
ADV = "vsdd-factory:adversary"

FINDING = {
    "key": "code:a",
    "title": "t",
    "severity": "high",
    "category": "code",
    "scope": "unit",
    "owner": "implementer",
    "location": {"path": "src/module.py", "line_start": 1, "line_end": 2},
    "spec_ref": "BC-1.00.001",
    "repro": "",
    "observed": "o",
    "expected": "e",
}

ROSTER = Path(__file__).resolve().parents[1] / "lint" / "roster.toml"
PLUGIN = Path(__file__).resolve().parents[1] / "plugins" / "vsdd-factory"


def _git(root: Path, *args: str) -> str:
    proc = subprocess.run(
        ["git", "-C", str(root), *args], check=True, capture_output=True, text=True
    )
    return proc.stdout.strip()


def _commit(root: Path, message: str) -> str:
    _git(root, "add", "-A")
    _git(root, "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", message)
    return _git(root, "rev-parse", "HEAD")


_COUNT = itertools.count()


def _work_commit(tree: Path, message: str = "work") -> str:
    """A real change in the story's worktree, then a commit; its head."""
    (Path(tree) / "work.txt").write_text(f"{message} {next(_COUNT)}\n", encoding="utf-8")
    return _commit(tree, message)


def _ready(tmp_path, *, product=None):
    """A story-ready project with the story's worktree, a live run and a
    closure whose ``delivery.base`` is the story's base."""
    root = story_ready.make_project(tmp_path, product=product)
    base = _git(root, "rev-parse", "develop")
    tree = root / ".worktrees" / SUBJECT
    subprocess.run(
        ["git", "-C", str(root), "worktree", "add", "-b", f"story/{SUBJECT}", str(tree), base],
        check=True,
        capture_output=True,
    )
    run = runs.run_open(root, workflow="story", cycle="c-1", session="sess", subject=SUBJECT)

    def set_delivery(record):
        delivery = record.setdefault("delivery", {})
        delivery["base"] = base
        delivery["head"] = _git(tree, "rev-parse", "HEAD")

    cycles.closure_update(root, "c-1", "story", SUBJECT, set_delivery)
    return root, tree, base, run["id"]


def _review_data(head, findings=None, *, coverage=None, checks=None):
    return {
        "reviewed_sha": head,
        "findings": findings or [],
        "claim_checks": checks or [],
        "coverage": coverage
        if coverage is not None
        else [{"item": "AC-1", "state": "checked", "note": "n"}],
    }


def _store(
    root,
    run_id,
    head,
    *,
    name="review-a-l1-p1",
    method=REVIEW,
    agent_type=ADV,
    attempt=1,
    loop_=1,
    pass_=1,
    status="done",
    data=None,
    gates=None,
    subject=SUBJECT,
    workflow="story",
):
    result = {
        "step": {
            "run": run_id,
            "workflow": workflow,
            "name": name,
            "subject": subject,
            "attempt": attempt,
            "method": method,
            "loop": loop_,
            "pass": pass_,
        },
        "status": status,
        "summary": "s",
        "head": head,
        "data": data if data is not None else {},
    }
    if gates is not None:
        result["gates"] = gates
    return store_step(root, result, agent_type, head)


def _clean_pass(root, run_id, head, *, pass_=1, loop_=1, agent_type=ADV, name=None, coverage=None):
    return _store(
        root,
        run_id,
        head,
        name=name or f"review-{'a' if agent_type == ADV else 'x'}-l{loop_}-p{pass_}",
        agent_type=agent_type,
        loop_=loop_,
        pass_=pass_,
        data=_review_data(head, coverage=coverage),
    )


# ------------------------------------------------------------- the facade ---


def test_flat_name_resolves():
    from vsdd import state
    from vsdd.state import verdicts as module

    assert state.verdict is module.verdict
    assert callable(module.changed_files) and callable(module.covering)
    assert callable(module.summary) and callable(module.waivers)
    assert callable(module.rounds_used) and callable(module.passes_used)
    assert callable(module.public_verdict) and callable(module.rebuilt) and callable(module.rebuild)


# -------------------------------------------------------- the empty story ---


def test_no_pass_is_failed_no_result(tmp_path):
    root, _tree, _base, _run = _ready(tmp_path)
    verdict = verdicts.verdict(root, SUBJECT)
    assert verdict["outcome"] == "failed" and verdict["reason"] == "no_result"
    assert verdict["passes"] == 0 and verdict["rounds_used"] == 0 and verdict["head"] == ""
    for field in ("blocking", "advisory", "deferred", "dismissed", "coverage_missing"):
        assert verdict[field] == []
    assert verdicts.summary(root, SUBJECT) is None
    assert verdicts.covering(root, SUBJECT) is None


# ------------------------------------------------- a stored clean pass ------


def test_stored_clean_pass_converges_and_covers(tmp_path):
    root, tree, _base, run = _ready(tmp_path)
    head = _work_commit(tree, "work\n")
    _clean_pass(root, run, head)
    verdict = verdicts.verdict(root, SUBJECT)
    assert verdict["outcome"] == "converged" and verdict["reason"] == "clean"
    assert verdict["head"] == head and verdict["passes"] == 1 and verdict["rounds_used"] == 1
    covered = verdicts.covering(root, SUBJECT)
    assert covered is not None and covered["head"] == head and covered["outcome"] == "converged"
    assert verdicts.summary(root, SUBJECT)["outcome"] == "converged"


def test_covering_none_after_a_new_commit(tmp_path):
    root, tree, _base, run = _ready(tmp_path)
    _clean_pass(root, run, _work_commit(tree, "work\n"))
    assert verdicts.covering(root, SUBJECT) is not None
    _work_commit(tree, "more work\n")
    assert verdicts.covering(root, SUBJECT) is None


def test_covering_by_patch_id(tmp_path):
    root, tree, _base, run = _ready(tmp_path)
    head = _work_commit(tree, "work\n")
    _clean_pass(root, run, head)
    subprocess.run(
        ["git", "-C", str(tree), "-c", "user.name=t", "-c", "user.email=t@t",
         "commit", "--amend", "-qm", "work, amended"],
        check=True,
        capture_output=True,
    )
    amended = _git(tree, "rev-parse", "HEAD")
    assert amended != head
    covered = verdicts.covering(root, SUBJECT)
    assert covered is not None, "the amended commit keeps the tree, so the patch id covers it"
    assert covered["head"] == head, "the closure form names the commit the pass reviewed"


def test_computed_overrules_closure(tmp_path):
    root, tree, base, run = _ready(tmp_path)
    head = _work_commit(tree, "work\n")

    def store_verdict(record):
        # summary()'s form: the stored verdict carries its own head and patch
        # id, by which it covers (S4R-verdicts-1).
        record["verdict"] = {
            "outcome": "converged",
            "passes": 2,
            "head": head,
            "patch_id": _patch_id(tree, base, head),
        }

    cycles.closure_update(root, "c-1", "story", SUBJECT, store_verdict)
    assert verdicts.covering(root, SUBJECT) is not None

    # A pass recorded after the checkpoint: the verdict about the commit asked
    # about wins, and it is not converged, so nothing covers (decision D8).
    _store(
        root,
        run,
        head,
        name="review-a-l1-p3",
        pass_=3,
        data=_review_data(head, findings=[dict(FINDING)]),
    )
    assert verdicts.verdict(root, SUBJECT)["outcome"] != "converged"
    assert verdicts.covering(root, SUBJECT) is None

    # And once the records converge again — the open claim resolved — the
    # computed verdict covers.
    _store(
        root,
        run,
        head,
        name="review-a-l1-p4",
        pass_=4,
        data=_review_data(head, checks=[{"key": "code:a", "outcome": "resolved"}]),
    )
    covered = verdicts.covering(root, SUBJECT)
    assert covered is not None and covered["head"] == head


def test_closure_used_without_records(tmp_path):
    """The closure's stored verdict covers by its own head, in summary()'s
    form and with no ``delivery.head`` — nothing writes that before a merge
    (S4R-verdicts-1)."""
    root, tree, base, _run = _ready(tmp_path)
    head = _work_commit(tree, "work\n")

    def store_verdict(record):
        record["verdict"] = {
            "perimeter": "story",
            "head": head,
            "patch_id": _patch_id(tree, base, head),
            "passes": 2,
            "rounds_used": 2,
            "outcome": "converged",
            "advisory": 0,
            "dismissed": 0,
            "waived": 0,
        }

    cycles.closure_update(root, "c-1", "story", SUBJECT, store_verdict)
    covered = verdicts.covering(root, SUBJECT)
    assert covered is not None and covered["outcome"] == "converged"
    assert covered["passes"] == 2, "the closure's stored verdict, with no records to compute one"


def test_closure_not_covered_after_a_new_commit(tmp_path):
    """The stale direction: a verdict converged at H1 does not cover H2, even
    though the closure's ``delivery.head`` names H2 (S4R-verdicts-1)."""
    root, tree, base, _run = _ready(tmp_path)
    h1 = _work_commit(tree, "work\n")

    def store_verdict(record):
        record["verdict"] = {
            "perimeter": "story",
            "head": h1,
            "patch_id": _patch_id(tree, base, h1),
            "passes": 2,
            "rounds_used": 2,
            "outcome": "converged",
            "advisory": 0,
            "dismissed": 0,
            "waived": 0,
        }

    cycles.closure_update(root, "c-1", "story", SUBJECT, store_verdict)
    assert verdicts.covering(root, SUBJECT) is not None
    h2 = _work_commit(tree, "more work\n")
    assert h2 != h1

    def store_head(record):
        record.setdefault("delivery", {})["head"] = h2

    cycles.closure_update(root, "c-1", "story", SUBJECT, store_head)
    assert verdicts.covering(root, SUBJECT, head=h2) is None, (
        "delivery.head names H2, but the stored verdict is about H1"
    )


def test_evidence_review_is_covering(tmp_path):
    """``evidence(project, "review", subject)`` is ``covering``: inside the run
    that stored the passes and before any checkpoint, the review lookup
    answers with the computed verdict (PL4-2)."""
    root, tree, _base, run = _ready(tmp_path)
    head = _work_commit(tree, "work\n")
    _clean_pass(root, run, head)
    from vsdd.state.gate_evidence import evidence

    covered = evidence(root, "review", SUBJECT)
    assert covered is not None and covered["head"] == head
    assert covered["outcome"] == "converged"


def test_chain_open_at_one_loop_keeps_covering_none(tmp_path):
    """``chain`` holds only when every loop from the last back to the nearest
    loop 1 converged (rule 9): a loop 1 left blocked, cap, and a later loop
    that converges give a converged verdict that still covers nothing
    (S4R-verdicts-3)."""
    root, tree, _base, run = _ready(tmp_path)
    head = _work_commit(tree, "work\n")

    def fix(pass_):
        _store(
            root,
            run,
            head,
            name=f"fix-i-l1-p{pass_}",
            method="fix-findings",
            agent_type="vsdd-factory:implementer",
            pass_=pass_,
            data={
                "resolutions": [
                    {"key": "code:a", "resolution": "fixed", "commit": "c1", "note": ""}
                ]
            },
            gates=[
                {
                    "gate": "green",
                    "subject": SUBJECT,
                    "ok": True,
                    "passed": True,
                    "sha": head,
                    "evidence": "e",
                    "failures": [],
                }
            ],
        )

    # Loop 1 runs to the cap with the finding open: fix, retry, arbiter, cap.
    _store(root, run, head, data=_review_data(head, findings=[dict(FINDING)]))
    fix(1)
    for pass_ in (2, 3):
        _store(
            root,
            run,
            head,
            name=f"review-a-l1-p{pass_}",
            pass_=pass_,
            data=_review_data(head, checks=[{"key": "code:a", "outcome": "not_resolved"}]),
        )
        fix(pass_)
    _store(
        root,
        run,
        head,
        name="review-a-l1-p4",
        pass_=4,
        data=_review_data(head, checks=[{"key": "code:a", "outcome": "not_resolved"}]),
    )
    rebuilt = verdicts.rebuilt(root, SUBJECT)
    assert [(loop["verdict"]["outcome"], loop["verdict"]["reason"]) for loop in rebuilt["loops"]] == [
        ("blocked", "cap")
    ], "loop 1 runs to the cap with the finding open"
    # A later loop converges on the same head: nothing changed, so its items
    # are the empty list.
    _clean_pass(root, run, head, name="review-a-l2-p1", loop_=2, pass_=1, coverage=[])
    verdict = verdicts.verdict(root, SUBJECT)
    assert verdict["outcome"] == "converged"
    assert verdicts.rebuilt(root, SUBJECT)["chain"] is False
    assert verdicts.covering(root, SUBJECT) is None, (
        "the verdict is the later loop's, but the chain is broken at loop 1"
    )


# ---------------------------------------------------------- items and ACs ---


def test_items_equal_unit_acs(tmp_path):
    """The verdict's items are the story's ``### AC-N`` headings, the ones
    ``story_unit.build`` reads (decision D9) — both readers skip the
    headings inside a code fence (S4R-verdicts-7). The one reader of the two
    lives in ``story_unit.acs`` and this unit's ``_story_acs`` calls it: a
    builder of ``story_unit.py`` (the survey-2 unit rewrites it) must keep
    that fence-skipping ``acs`` (S4R-verdicts-C1-4)."""
    root, tree, _base, run = _ready(tmp_path)
    story = root / ".factory" / "stories" / "E-9" / f"{SUBJECT}.md"
    story.write_text(
        "\n".join(
            [
                "---",
                f"id: {SUBJECT}",
                "title: Sample story",
                "status: draft",
                "cycle: c-1",
                "kind: fix",
                "priority: P1",
                "criticality: medium",
                "contracts: [BC-1.00.001]",
                "demo: false",
                "---",
                "",
                "## Acceptance criteria",
                "",
                "### AC-1 (BC-1.00.001)",
                "",
                "The sample behaves as the contract says.",
                "",
                "```",
                "a fenced sample of the contract's wording, no real heading here:",
                "### AC-9 (not a criterion, inside the fence)",
                "```",
                "",
                "### AC-2 (BC-1.00.001)",
                "",
                "And the other thing.",
                "",
            ]
        ),
        encoding="utf-8",
    )
    head = _work_commit(tree, "work\n")
    _clean_pass(root, run, head, coverage=[])
    from vsdd.state import story_unit

    unit = story_unit.build(root, "c-1", SUBJECT)
    verdict = verdicts.verdict(root, SUBJECT)
    assert verdict["coverage_missing"] == unit["acs"] == ["AC-1", "AC-2"]


# ------------------------------------------------------------- changed -----.


def test_changed_files_and_later_loop_items(tmp_path):
    """``changed_files`` is the one definition of "the files changed" (VSA-03):
    ``--no-renames`` so a renamed file gives both names, sorted, and ``None``
    when git cannot say. A later loop's items are the files this returns
    between the loop before it and the loop's own first pass (decision D7)."""
    root = tmp_path / "proj"
    root.mkdir()
    _git(root, "init", "-q", "-b", "develop")
    _git(root, "config", "user.email", "t@t")
    _git(root, "config", "user.name", "t")
    (root / "src.py").write_text("one\n", encoding="utf-8")
    (root / "moved.py").write_text("keep\n", encoding="utf-8")
    _commit(root, "base")

    (root / "src.py").write_text("two\n", encoding="utf-8")
    c1 = _commit(root, "edit src")
    (root / "new.py").write_text("added\n", encoding="utf-8")
    _git(root, "mv", "moved.py", "renamed.py")
    accent_file = root / "café.py"
    accent_file.write_text("accents\n", encoding="utf-8")
    (root / 'quo"te.py').write_text("quoted\n", encoding="utf-8")
    c2 = _commit(root, "rename, add, accents and a quote")

    accent = accent_file.name  # the spelling the filesystem kept
    files = verdicts.changed_files(root, c1, c2)
    assert files == sorted(["moved.py", "new.py", "renamed.py", accent, 'quo"te.py']), (
        "the rename gives both names, sorted"
    )
    assert 'quo"te.py' in files, "the double quote arrives unquoted"
    normalized = [unicodedata.normalize("NFC", name) for name in files]
    assert "café.py" in normalized, "the accented name arrives unquoted"
    assert not any(name.startswith('"') for name in files), (
        "git's C-quoting would wrap unusual names in double quotes"
    )
    assert verdicts.changed_files(root, c2, c1) == files
    assert verdicts.changed_files(root, c1, c1) == []
    assert verdicts.changed_files(root, c1, "0" * 40) is None
    assert verdicts.changed_files(root, "", c2) is None

    # The later loop's items: between the commit the loop before it converged
    # on (c1) and the head its first pass reviewed (c2), the renamed file is
    # two items, so a reviewer can be asked about both names.
    (root / "renamed.py").unlink()
    _commit(root, "lose renamed")
    later = verdicts.changed_files(root, c1, _git(root, "rev-parse", "HEAD"))
    assert later == sorted(["moved.py", "new.py", accent, 'quo"te.py']), (
        "a rename inside the range keeps both names; a file added and deleted "
        "inside the range is not"
    )


# ---------------------------------------------------------------- waivers ---


def test_waivers_from_settled_decision(tmp_path):
    root, _tree, _base, _run = _ready(tmp_path)
    from vsdd.state import decisions as decisions_store

    decisions_store.decision_open(root, cycle="c-1", kind="finding", packet="# finding",
                  subject=SUBJECT)
    decisions_store.settle_decision(root, f"finding.{SUBJECT}", "waive",
                    [("key", "code:a"), ("key", "code:b")], "noisy", story_ready.SESSION)
    assert verdicts.waivers(root, SUBJECT) == ["code:a", "code:b"]

    # A second finding decision replaces the record, so the settled keys must
    # be carried again or they are lost (ORDER.md section 4, step 4 note).
    decisions_store.decision_open(root, cycle="c-1", kind="finding", packet="# finding again",
                  subject=SUBJECT)
    decisions_store.settle_decision(root, f"finding.{SUBJECT}", "waive-coverage",
                    [("item", "AC-1")], "covered elsewhere", story_ready.SESSION)
    assert verdicts.waivers(root, SUBJECT) == ["coverage:S-9.01:AC-1"]
    # Unknown subjects have no waivers and no decision read errors.
    assert verdicts.waivers(root, "S-absent") == []


def test_waived_verdict_converges_through_the_decision(tmp_path):
    """The settled waiver reaches the verdict through ``waivers`` (P3OPS-09):
    the same records stay uncapped without it and read reviewed with it."""
    root, tree, _base, run = _ready(tmp_path)
    head = _work_commit(tree, "work\n")
    _store(root, run, head, data=_review_data(head, findings=[dict(FINDING)]))
    assert verdicts.verdict(root, SUBJECT)["outcome"] != "converged", (
        "the finding is open while nothing waives it"
    )
    from vsdd.state import decisions as decisions_store

    decisions_store.decision_open(
        root, cycle="c-1", kind="finding", packet="# finding", subject=SUBJECT
    )
    decisions_store.settle_decision(
        root, f"finding.{SUBJECT}", "waive", [("key", "code:a")], "noisy", story_ready.SESSION
    )
    verdict = verdicts.verdict(root, SUBJECT)
    assert verdict["outcome"] == "converged" and verdict["reason"] == "clean"
    covered = verdicts.covering(root, SUBJECT)
    assert covered is not None and covered["outcome"] == "converged"


# ------------------------------------------------------------ used rounds ---


def test_passes_used_counts_run_and_pass_pairs(tmp_path):
    root, tree, _base, run1 = _ready(tmp_path)
    for pass_ in (1, 2, 3):
        head = _work_commit(tree, f"work {pass_}")
        _clean_pass(root, run1, head, pass_=pass_)
    run2 = runs.run_open(root, workflow="story", cycle="c-1", session="sess", subject=SUBJECT)
    head = _work_commit(tree, "work 4\n")
    _clean_pass(root, run2["id"], head)
    assert verdicts.passes_used(root, SUBJECT) == 4, "3 and 1 give 4"


def test_rounds_used_never_below_closure(tmp_path):
    root, tree, _base, run = _ready(tmp_path)
    # No run records: the closure's stored count holds the floor.
    def store_verdict(record):
        record["verdict"] = {"outcome": "converged", "passes": 2, "rounds_used": 2}

    cycles.closure_update(root, "c-1", "story", SUBJECT, store_verdict)
    assert verdicts.rounds_used(root, SUBJECT) == 2

    # A last loop that is a later loop: the loop-1 count stays 0 and the
    # stored closure count still holds the floor (decision D13).
    head = _work_commit(tree, "work\n")
    _clean_pass(root, run, head, loop_=2, pass_=1)
    assert verdicts.passes_used(root, SUBJECT) == 0
    assert verdicts.rounds_used(root, SUBJECT) == 2
    assert verdicts.summary(root, SUBJECT)["rounds_used"] == 2

    # Run records past the stored count raise it.
    _clean_pass(root, run, head, pass_=1)
    _clean_pass(root, run, head, pass_=2)
    _clean_pass(root, run, head, pass_=3)
    assert verdicts.rounds_used(root, SUBJECT) == 3


def test_storing_the_summary_moves_nothing(tmp_path):
    """Storing ``summary()`` as the closure's verdict — what the checkpoint
    does — leaves the verdict, the closure form and the cover exactly where
    they were (S4R-verdicts-C1-1): the replay's ``used`` is the loop-1 passes
    of earlier runs (rule 9), so the stored count raises nothing and the
    arbiter stays on its pass (S4R-verdicts-15)."""
    root, tree, _base, run = _ready(tmp_path)
    head = _work_commit(tree, "work\n")

    def fix(pass_):
        _store(
            root,
            run,
            head,
            name=f"fix-i-l1-p{pass_}",
            method="fix-findings",
            agent_type="vsdd-factory:implementer",
            data={
                "resolutions": [
                    {"key": "code:a", "resolution": "disputed", "commit": "", "note": "because"}
                ]
            },
            gates=[
                {
                    "gate": "green",
                    "subject": SUBJECT,
                    "ok": True,
                    "passed": True,
                    "sha": head,
                    "evidence": "e",
                    "failures": [],
                }
            ],
        )

    # The finding is disputed at pass 1 and the arbiter rules not_defect at
    # pass 3, where the cap falls: converged, clean, dismissed.
    _store(root, run, head, data=_review_data(head, findings=[dict(FINDING)]))
    fix(1)
    _store(
        root,
        run,
        head,
        name="review-a-l1-p2",
        pass_=2,
        data=_review_data(head, checks=[{"key": "code:a", "outcome": "not_resolved"}]),
    )
    fix(2)
    _store(
        root,
        run,
        head,
        name="review-a-l1-p3",
        pass_=3,
        data=_review_data(head, checks=[{"key": "code:a", "outcome": "not_resolved"}]),
    )
    _store(
        root,
        run,
        head,
        name="arbitrate-u-l1",
        method="arbitrate",
        agent_type="vsdd-factory:architect",
        data={"rulings": [{"key": "code:a", "ruling": "not_defect", "reason": "r"}]},
    )
    verdict = verdicts.verdict(root, SUBJECT)
    assert verdict["outcome"] == "converged" and verdict["reason"] == "clean"
    assert verdict["passes"] == 3 and verdict["rounds_used"] == 3
    assert [entry["key"] for entry in verdict["dismissed"]] == ["code:a"]
    rebuilt = verdicts.rebuilt(root, SUBJECT)
    assert rebuilt["loops"][0]["input"]["used"] == 0, "rule 9: only earlier runs' passes"
    assert rebuilt["chain"] is True
    before = (verdict, verdicts.summary(root, SUBJECT), verdicts.covering(root, SUBJECT))
    assert before[1]["rounds_used"] == 3
    assert before[2] is not None

    # The checkpoint stores the computed summary as the closure's verdict;
    # the next computation must read the same records the same way.
    stored = before[1]
    cycles.closure_update(
        root, "c-1", "story", SUBJECT, lambda record: record.update(verdict=stored)
    )
    after = (
        verdicts.verdict(root, SUBJECT),
        verdicts.summary(root, SUBJECT),
        verdicts.covering(root, SUBJECT),
    )
    assert after == before, "the stored count moves neither the verdict nor the cover"
    assert verdicts.rebuilt(root, SUBJECT)["loops"][0]["input"]["used"] == 0


def test_summary_is_valid_closure_verdict(tmp_path):
    root, tree, base, run = _ready(tmp_path)
    head = _work_commit(tree, "work\n")
    _clean_pass(root, run, head)
    summary = verdicts.summary(root, SUBJECT)
    assert summary == {
        "perimeter": "story",
        "head": head,
        "patch_id": _patch_id(tree, base, head),
        "passes": 1,
        "rounds_used": 1,
        "outcome": "converged",
        "advisory": 0,
        "dismissed": 0,
        "waived": 0,
    }
    from vsdd.schema import is_valid, load_registry

    registry = load_registry(SCHEMA_DIR)
    assert is_valid(
        summary, registry["closure-record.schema.json"]["properties"]["verdict"], registry
    ), "the closure form is valid against the closure record's verdict"


def _patch_id(tree, base, head):
    from vsdd.state.gate_evidence import patch_id

    return patch_id(tree, base, head)


# --------------------------------------------------------------- grounding --


def test_verdict_trusts_stored_mark(tmp_path):
    """The verdict trusts the stored ``grounded`` mark (VSA-07, decision D5):
    a finding marked ``grounded: false`` is advisory whatever its severity, in
    the worktree as it stands now."""
    root, tree, _base, run = _ready(tmp_path)
    head = _work_commit(tree, "work\n")
    finding = dict(FINDING, severity="critical", grounded=False)
    _store(root, run, head, data=_review_data(head, findings=[finding]))
    verdict = verdicts.verdict(root, SUBJECT)
    assert verdict["blocking"] == [] and [f["key"] for f in verdict["advisory"]] == ["code:a"]


def test_verdict_never_regounds_an_unmarked_finding(tmp_path):
    """An unmarked finding keeps blocking after its file is gone (VSA-07):
    re-grounding against the worktree as it stands now would unground it."""
    root, tree, _base, run = _ready(tmp_path)
    src = tree / "src" / "module.py"
    src.parent.mkdir(exist_ok=True)
    src.write_text("def a():\n    pass\n", encoding="utf-8")
    head = _commit(tree, "the finding's file")
    _store(root, run, head, data=_review_data(head, findings=[dict(FINDING, severity="critical")]))
    src.unlink()
    _commit(tree, "the file is gone")
    verdict = verdicts.verdict(root, SUBJECT)
    assert [f["key"] for f in verdict["blocking"]] == ["code:a"], (
        "no stored mark, so the finding is treated as grounded, whatever the "
        "worktree holds now"
    )


# ------------------------------------------------------------ loud failures --


def test_corrupt_step_record_raises(tmp_path):
    """A step record that does not parse raises (decision D14): skipping it
    could turn an open loop into a converged one."""
    root, tree, _base, run = _ready(tmp_path)
    head = _work_commit(tree, "work\n")
    _clean_pass(root, run, head)
    bad = root / ".factory" / "local" / "runs" / run / "steps" / "story" / SUBJECT / "bad.1.json"
    bad.write_text("{not json", encoding="utf-8")
    with pytest.raises(StoreCorrupt):
        verdicts.verdict(root, SUBJECT)


def test_unknown_commit_is_not_converged(tmp_path):
    """A verdict about a commit git cannot see covers nothing: the lookup
    answers None and the closure form cannot compute a patch id."""
    root, tree, base, run = _ready(tmp_path)
    unknown = "0" * 40
    _store(root, run, unknown, data=_review_data(unknown))
    verdict = verdicts.verdict(root, SUBJECT)
    assert verdict["outcome"] == "converged" and verdict["head"] == unknown
    assert verdicts.covering(root, SUBJECT) is None
    assert verdicts.summary(root, SUBJECT)["patch_id"] == ""


def test_uncounted_agent_type_is_listed_in_skipped(tmp_path):
    """A pass of any other type than the method's owner is not counted, and
    the tool says so in ``rebuilt()['skipped']`` (platform item 36)."""
    root, tree, _base, run = _ready(tmp_path)
    head = _work_commit(tree, "work\n")
    _store(root, run, head, agent_type="vsdd-factory:implementer", data=_review_data(head))
    rebuilt = verdicts.rebuilt(root, SUBJECT)
    assert rebuilt["verdict"]["passes"] == 0
    assert {"run": run, "step": "review-a-l1-p1.1", "why": "agent_type"} in rebuilt["skipped"]


def test_public_verdict_hides_walled(tmp_path, monkeypatch):
    """A walled finding — one under ``.factory/holdout/`` or returned by the
    scenario reviewer — is replaced by its walled key, severity and the mark;
    records itself refuses to store one (plan step-4 verdicts.md 7)."""
    root, tree, _base, run = _ready(tmp_path)
    head = _work_commit(tree, "work\n")
    walled = dict(FINDING, key="holdout:HS-9", severity="medium",
                  location={"path": ".factory/holdout/scores.json"})
    normal = dict(FINDING, key="code:n", severity="medium")
    stored = _store(root, run, head, data=_review_data(head, findings=[normal]))

    # The store refuses a finding whose location lies under the wall.
    with pytest.raises(StoreRefused) as exc:
        _store(
            root,
            run,
            head,
            name="review-a-l1-p2",
            pass_=2,
            data=_review_data(head, findings=[walled]),
        )
    assert exc.value.code == "walled_not_built"

    # Through the pure rebuild: the verdict is computed from the stored record
    # adjusted to carry the walled finding, which is the only way one reaches
    # the tool today, and public_verdict replaces exactly that one.
    import json

    record = json.loads(json.dumps(stored))
    record["data"] = _review_data(head, findings=[walled, normal])
    result = verdicts.rebuild(
        [record], subject=SUBJECT, cap=3, waived=[], acs=["AC-1"], changed=lambda a, b: None
    )
    assert result["walled"] == ["holdout:HS-9"]
    monkeypatch.setattr(verdicts, "_compute", lambda project, subject: (result["verdict"], result))
    public = verdicts.public_verdict(root, SUBJECT)
    digest = hashlib.sha256(b"holdout:HS-9").hexdigest()[:12]
    assert public["advisory"] == [
        {"key": f"walled:{digest}", "severity": "medium", "walled": True},
        normal,
    ]

    # A walled finding the reviewer withdrew is dismissed by key in clear,
    # with the reason beside it; public_verdict maps the key and drops the
    # reason (S4R-verdicts-11).
    walled_high = dict(walled, severity="high")
    first = json.loads(json.dumps(stored))
    first["data"] = _review_data(head, findings=[walled_high])
    second = json.loads(json.dumps(stored))
    second["step"]["name"] = "review-a-l1-p2"
    second["step"]["pass"] = 2
    second["data"] = _review_data(
        head,
        findings=[],
        checks=[{"key": "holdout:HS-9", "outcome": "withdrawn"}],
    )
    result = verdicts.rebuild(
        [first, second], subject=SUBJECT, cap=3, waived=[], acs=["AC-1"], changed=lambda a, b: None
    )
    assert result["verdict"]["outcome"] == "converged"
    assert result["verdict"]["dismissed"] == [
        {"key": "holdout:HS-9", "by": "reviewer", "reason": "Withdrawn on re-review."}
    ]
    monkeypatch.setattr(verdicts, "_compute", lambda project, subject: (result["verdict"], result))
    public = verdicts.public_verdict(root, SUBJECT)
    assert public["dismissed"] == [
        {"key": f"walled:{digest}", "by": "reviewer", "reason": ""}
    ], "the withdrawn walled key is hidden with its reason"


# ------------------------------------------------------------ the tables ----


def test_review_methods_table():
    """``REVIEW_METHODS``' owners are reviewer-tier agents and the arbiter's
    owner is the planner-tier architect (decision D2); where an agent file
    exists, it lists its method."""
    roster = tomllib.loads(ROSTER.read_text(encoding="utf-8"))
    agents = roster["agents"]
    for method, (owner, _accounts) in verdicts.REVIEW_METHODS.items():
        assert agents.get(owner) == "reviewer", method
        _agent_file_lists(owner, method)
    arbiter_owner = verdicts.ARBITER[1]
    assert agents.get(arbiter_owner) == "planner"
    _agent_file_lists(arbiter_owner, verdicts.ARBITER[0])


def _agent_file_lists(owner: str, method: str) -> None:
    file = PLUGIN / "agents" / f"{owner}.md"
    if file.exists():
        assert method in file.read_text(encoding="utf-8"), (
            f"{owner}.md does not list the method {method!r}"
        )
