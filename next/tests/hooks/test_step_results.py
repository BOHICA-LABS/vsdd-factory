"""The step-results rule in process (plan rules.md section 7): storing, the
gate entries against the evidence store, the once-per-step grounding block,
the mark, and the refusals."""

from __future__ import annotations

import json
import os
import stat

from tests.hooks import rule_projects
from tests.hooks.runner_helpers import run_hook
from tests.hooks.test_runner import base_env
from vsdd.hooks.gitfs import head_sha
from vsdd.hooks.model import RuleEntry

ROW = RuleEntry(
    id="step-results",
    on=(("PreToolUse", ("StructuredOutput",)),),
    kind="gate",
    fail="closed",
    locked=True,
    rewrites=True,
    summary="fixture row for the step-results tests",
)


def call(p, result, *, agent_type="vsdd-factory:implementer", agent_id="agent-7"):
    payload = {
        "hook_event_name": "PreToolUse",
        "tool_name": "StructuredOutput",
        "session_id": "sess-rules",
        "agent_type": agent_type,
        "agent_id": agent_id,
        "cwd": str(p.root),
        "tool_input": result,
    }
    code, out, err = run_hook(payload, "gate", entries=(ROW,), env=base_env(p.root))
    rendered = json.loads(out) if out.strip() else None
    return code, err, rendered


def a_result(p, **changes):
    result = rule_projects.step_result(p)
    result.update(changes)
    return result


def stored(p, attempt=1):
    from vsdd.state.steps import step_read

    return step_read(p.root, p.run, f"story/{p.story}/implement.{attempt}")


def events(p, kind):
    found = []
    directory = p.root / "events"
    if not directory.is_dir():
        return found
    for path in sorted(directory.rglob("*.jsonl")):
        for line in path.read_text(encoding="utf-8").splitlines():
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(event, dict) and event.get("type") == kind:
                found.append(event)
    return found


def test_stores_with_agent_type_and_head(tmp_path):
    p = rule_projects.story_project(tmp_path)
    head = head_sha(p.worktree)
    result = a_result(p)
    code, err, _rendered = call(p, result)
    assert code == 0, err

    record = stored(p)
    assert record is not None, err
    assert record["agent_type"] == "vsdd-factory:implementer"
    assert record["agent_id"] == "agent-7"
    assert record["worktree_head"] == head
    assert record["status"] == "done"

    recorded = events(p, "step.recorded")
    assert any(
        event.get("run") == p.run and event.get("name") == "implement" for event in recorded
    ), recorded


def test_no_identity_passes(tmp_path):
    p = rule_projects.story_project(tmp_path)
    relayed = {"relayed": {"by": "clerk", "step": {"workflow": "story"}}}  # no step.run
    code, err, rendered = call(p, relayed, agent_type="vsdd-factory:clerk")
    assert code == 0, err
    assert rendered is None  # nothing said, nothing stored
    assert stored(p) is None


def gate_entry(p, **changes):
    entry = {
        "ok": True,
        "gate": "green",
        "subject": p.story,
        "sha": head_sha(p.worktree),
        "passed": True,
        "evidence": ".factory/local/evidence/x.json",
        "failures": [],
    }
    entry.update(changes)
    return entry


def test_gate_entries(tmp_path):
    p = rule_projects.story_project(tmp_path)
    entry = gate_entry(p)

    # A passing entry with no record at the store is blocked, and the forged
    # evidence path it carries is never opened (decision D11): the record is
    # looked up by subject, gate and commit alone.
    forged = dict(entry, evidence="../../etc/passwd")
    code, err, _r = call(p, a_result(p, gates=[forged]))
    assert code == 10, err
    assert "gate_result_mismatch" in err, err
    assert stored(p) is None

    # An entry whose commit differs from its record's is blocked.
    rule_projects.green(p)
    code, err, _r = call(p, a_result(p, gates=[dict(entry, sha="a" * 40)]))
    assert code == 10, err
    assert "gate_result_mismatch" in err, err

    # A matching entry is stored; a failing entry needs no record.
    code, err, _r = call(p, a_result(p, gates=[entry, dict(entry, passed=False)]))
    assert code == 0, err
    assert stored(p) is not None, err


def test_forged_evidence_path_is_ignored(tmp_path):
    """VSA-11: the record is looked up by subject, gate and commit alone. A
    forged record inside the project, named by the entry's evidence path,
    never makes a gate result pass while the store has nothing
    (S4R-rules-14)."""
    p = rule_projects.story_project(tmp_path)
    forged = p.root / ".factory" / "local" / "forged-evidence"
    forged.parent.mkdir(parents=True, exist_ok=True)
    forged.write_text(
        json.dumps({
            "schema": 1, "gate": "demo", "subject": p.story,
            "sha": head_sha(p.worktree), "passed": True,
        }),
        encoding="utf-8",
    )
    entry = gate_entry(p, gate="demo", evidence=str(forged))
    code, err, _r = call(p, a_result(p, gates=[entry]))
    assert code == 10, err
    assert "gate_result_mismatch" in err, err
    assert stored(p) is None


def test_ungrounded_marks_by_position(tmp_path):
    """The mark lands on the failing findings by their position, whatever
    their keys: none, an integer, two the same (S4R-rules-8)."""
    p = rule_projects.story_project(tmp_path)
    target = p.worktree / "src" / "main.rs"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text("pub fn one() {}\n", encoding="utf-8")

    def ungrounded(title, key, quote):
        finding = {
            "title": title, "severity": "low", "category": "code",
            "scope": "unit", "owner": "implementer", "spec_ref": "", "repro": "",
            "observed": "o", "expected": "e",
            "location": {"path": "src/main.rs", "quote": quote},
        }
        if key is not None:
            finding["key"] = key
        return finding

    findings = [
        ungrounded("no key", None, "absent one"),
        ungrounded("integer key", 7, "absent two"),
        ungrounded("shared key", "same", "absent three"),
        ungrounded("shared key again", "same", "absent four"),
    ]
    result = a_result(p, data={"findings": findings})
    code, err, _r = call(p, result)
    assert code == 10, err
    assert "finding ?:" in err and "finding 7:" in err, err
    assert err.count("finding same:") == 2, err
    assert stored(p) is None

    # The second return is stored with every failing finding marked.
    code, err, _r = call(p, result)
    assert code == 0, err
    record = stored(p)
    assert [f.get("grounded") for f in record["data"]["findings"]] == [False] * 4


def test_ungrounded_returned_beside_other_blocks(tmp_path):
    """A gate mismatch and ungrounded findings come back in one return, the
    marker written, so the corrected return spends no extra platform retry
    (plan 3.4; S4R-rules-9)."""
    p = rule_projects.story_project(tmp_path)
    target = p.worktree / "src" / "main.rs"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text("pub fn one() {}\n", encoding="utf-8")
    finding = {
        "key": "code:drift", "title": "Drift", "severity": "low", "category": "code",
        "scope": "unit", "owner": "implementer", "spec_ref": "", "repro": "",
        "observed": "o", "expected": "e",
        "location": {"path": "src/main.rs", "quote": "absent"},
    }
    result = a_result(p, gates=[gate_entry(p)], data={"findings": [finding]})
    code, err, _r = call(p, result)
    assert code == 10, err
    assert "gate_result_mismatch" in err, err
    assert "findings_ungrounded" in err, err
    assert stored(p) is None

    # The gate entry repaired: the marked store happens without another
    # ungrounded round trip, the marker standing from the first return.
    rule_projects.green(p)
    code, err, _r = call(p, result)
    assert code == 0, err
    record = stored(p)
    assert record is not None, err
    assert record["data"]["findings"][0]["grounded"] is False


def test_ungrounded_blocks_once_then_marks(tmp_path):
    p = rule_projects.story_project(tmp_path)
    target = p.worktree / "src" / "main.rs"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text("pub fn one() {}\n", encoding="utf-8")
    finding = {
        "key": "code:drift",
        "title": "Drift",
        "severity": "low",
        "category": "code",
        "scope": "unit",
        "owner": "implementer",
        "location": {"path": "src/main.rs", "line_start": 1, "line_end": 1,
                     "quote": "entirely absent text"},
        "spec_ref": "",
        "repro": "",
        "observed": "o",
        "expected": "e",
    }
    result = a_result(p, data={"findings": [finding]})
    code, err, rendered = call(p, result)
    assert code == 10, err
    assert "finding code:drift:" in err, err
    assert err.count("finding code:drift:") == 1, err
    assert stored(p) is None  # nothing stored before the mark

    # The second return is stored with the mark, and the rewrite carries it
    # to the script beside an allow.
    code, err, rendered = call(p, result)
    assert code == 0, err
    record = stored(p)
    assert record is not None, err
    assert record["data"]["findings"][0]["grounded"] is False
    assert rendered is not None
    specifics = rendered["hookSpecificOutput"]
    assert specifics["permissionDecision"] == "allow"
    marked = specifics["updatedInput"]["data"]["findings"][0]["grounded"]
    assert marked is False

    # A corrected return grounds, stores unmarked, and is not rewritten.
    p2 = rule_projects.story_project(tmp_path)
    (p2.worktree / "src").mkdir(parents=True, exist_ok=True)
    (p2.worktree / "src" / "main.rs").write_text("pub fn one() {}\n", encoding="utf-8")
    good = dict(finding, location=dict(finding["location"], quote="pub fn one()"))
    result2 = a_result(p2, data={"findings": [good]})
    code, err, rendered = call(p2, result2)
    assert code == 0, err
    record2 = stored(p2)
    assert record2 is not None, err
    assert "grounded" not in record2["data"]["findings"][0]
    assert rendered is None


def test_stored_mark_is_the_only_grounding(tmp_path):
    p = rule_projects.story_project(tmp_path)
    target = p.worktree / "src" / "main.rs"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text("pub fn one() {}\n", encoding="utf-8")
    finding = {
        "key": "code:self",
        "title": "Self-declared",
        "severity": "low",
        "category": "code",
        "scope": "unit",
        "owner": "implementer",
        "location": {"path": "src/main.rs", "quote": "pub fn one()"},
        "spec_ref": "", "repro": "", "observed": "o", "expected": "e",
        "grounded": True,  # the agent's own word is not stored (VSA-07)
    }
    code, err, rendered = call(p, a_result(p, data={"findings": [finding]}))
    assert code == 0, err
    record = stored(p)
    assert record is not None, err
    assert "grounded" not in record["data"]["findings"][0], record
    # The rewrite tells the script what was stored (decision D13) — the
    # marked input, not the store's record with the runner's fields added
    # (plan 3.4; S4R-rules-24).
    assert rendered is not None
    assert "grounded" not in rendered["hookSpecificOutput"]["updatedInput"]["data"]["findings"][0]
    assert "agent_type" not in rendered["hookSpecificOutput"]["updatedInput"], rendered
    assert record["agent_type"] == "vsdd-factory:implementer"


def test_grounding_uses_the_unit_tree(tmp_path):
    p = rule_projects.story_project(tmp_path)
    # A worktree path and a .factory/ path ground; holdout is not checked.
    (p.worktree / "src").mkdir(parents=True, exist_ok=True)
    (p.worktree / "src" / "main.rs").write_text("one\n", encoding="utf-8")
    story_file = p.root / ".factory" / "stories" / "E-9" / f"{p.story}.md"
    holdout_finding = {
        "key": "holdout:silent", "category": "holdout", "title": "h",
        "severity": "low", "scope": "unit", "owner": "implementer",
        "location": {"path": "nowhere/at/all"}, "spec_ref": "", "repro": "",
        "observed": "o", "expected": "e",
    }
    result = a_result(
        p,
        data={"findings": [
            {"key": "code:wt", "category": "code", "title": "t", "severity": "low",
             "scope": "unit", "owner": "implementer", "spec_ref": "", "repro": "",
             "observed": "o", "expected": "e",
             "location": {"path": "src/main.rs", "quote": "one"}},
            {"key": "code:factory", "category": "code", "title": "t", "severity": "low",
             "scope": "spec", "owner": "implementer", "spec_ref": "", "repro": "",
             "observed": "o", "expected": "e",
             "location": {"path": str(story_file.relative_to(p.root)),
                          "quote": "Acceptance criteria"}},
            holdout_finding,
        ]},
    )
    code, err, _r = call(p, result)
    assert code == 0, err  # all three ground or are exempt

    # An unbuilt workflow has no tree: a finding with a path is tree_unknown.
    other = a_result(p)
    other["step"] = dict(other["step"], workflow="release")
    other["data"] = {"findings": [{
        "key": "code:any", "category": "code", "title": "t", "severity": "low",
        "scope": "unit", "owner": "implementer", "spec_ref": "", "repro": "",
        "observed": "o", "expected": "e", "location": {"path": "src/main.rs"},
    }]}
    code, err, _r = call(p, other)
    assert code == 10, err
    assert "tree_unknown" in err, err


def test_ended_run_blocks(tmp_path):
    p = rule_projects.story_project(tmp_path)
    from vsdd.state.runs import run_end

    run_end(p.root, p.run)
    code, err, _r = call(p, a_result(p))
    assert code == 10, err
    assert "run_not_live" in err, err
    assert stored(p) is None


def test_invalid_result_blocked_not_stored(tmp_path):
    p = rule_projects.story_project(tmp_path)
    # status outside the enum: the store's own validation refuses (P3OPS-21,
    # decision D14), and the rule blocks instead of storing it.
    code, err, _r = call(p, a_result(p, status="fine"))
    assert code == 10, err
    assert "schema_invalid" in err, err
    assert stored(p) is None


def test_store_refusals_block(tmp_path):
    p = rule_projects.story_project(tmp_path)
    # Over the cap: a data blob no field cap catches, past the record's 64 kB.
    code, err, _r = call(p, a_result(p, data={"blob": "x" * 70_000}))
    assert code == 10, err
    assert "over_cap" in err, err
    assert stored(p) is None
    # Another agent's key: one stored result, the second agent refused.
    mine = a_result(p)
    code, err, _r = call(p, mine, agent_id="agent-one")
    assert code == 0, err
    code, err, _r = call(p, a_result(p, summary="a different result"), agent_id="agent-two")
    assert code == 10, err
    assert "step_exists" in err, err


def test_walled_refused(tmp_path):
    p = rule_projects.story_project(tmp_path)
    finding = {
        "key": "holdout:leak", "title": "Leak", "severity": "low", "category": "code",
        "scope": "unit", "owner": "implementer", "spec_ref": "", "repro": "",
        "observed": "o", "expected": "e",
        "location": {"path": ".factory/holdout/scenarios/s2.md"},
    }
    # Dot segments do not carry a path out of the wall (S4R-rules-25), and
    # the state-relative spelling is walled too.
    walled_paths = (
        ".factory/holdout/scenarios/s2.md",
        ".factory/./holdout/scenarios/s2.md",
        "holdout/scenarios/s2.md",
    )
    for path in walled_paths:
        for agent in ("vsdd-factory:implementer", "vsdd-factory:scenario-reviewer"):
            fresh = rule_projects.story_project(tmp_path)
            walled = dict(finding, location={"path": path})
            code, err, _r = call(fresh, a_result(fresh, data={"findings": [walled]}),
                                 agent_type=agent)
            assert code == 10, (path, agent, err)
            assert "walled_not_built" in err, (path, agent, err)
            # And again: the refusal is not once-per-step.
            code, err, _r = call(fresh, a_result(fresh, data={"findings": [walled]}),
                                 agent_type=agent)
            assert code == 10, (path, agent, err)
            assert "walled_not_built" in err, (path, agent, err)
    # The scenario-reviewer's findings are walled as a whole.
    reviewer = rule_projects.story_project(tmp_path)
    benign = dict(finding, location={"path": "src/main.rs"}, key="code:benign")
    code, err, _r = call(reviewer, a_result(reviewer, data={"findings": [benign]}),
                         agent_type="vsdd-factory:scenario-reviewer")
    assert code == 10, err
    assert "walled_not_built" in err, err


def test_normalised_path_outside_the_wall_stores(tmp_path):
    """``.factory/holdout/../src/x`` names ``.factory/src/x``: outside the
    wall, so it is not refused as walled — the finding is grounded against
    the project root like any ``.factory/`` path (S4R-rules-25)."""
    p = rule_projects.story_project(tmp_path)
    cited = p.root / ".factory" / "src" / "x.md"
    cited.parent.mkdir(parents=True, exist_ok=True)
    cited.write_text("acceptance noted\n", encoding="utf-8")
    finding = {
        "key": "code:normalised", "title": "N", "severity": "low", "category": "code",
        "scope": "unit", "owner": "implementer", "spec_ref": "", "repro": "",
        "observed": "o", "expected": "e",
        "location": {"path": ".factory/holdout/../src/x.md",
                     "quote": "acceptance noted"},
    }
    result = a_result(p, data={"findings": [finding]})
    code, err, _r = call(p, result)
    assert code == 0, err
    assert "walled_not_built" not in err, err
    assert "findings_ungrounded" not in err, err  # the quote is found
    record = stored(p)
    assert record is not None, err
    assert "grounded" not in record["data"]["findings"][0]


def test_marker_unwritable_blocks(tmp_path):
    p = rule_projects.story_project(tmp_path)
    target = p.worktree / "src" / "main.rs"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text("pub fn one() {}\n", encoding="utf-8")
    finding = {
        "key": "code:drift", "title": "Drift", "severity": "low", "category": "code",
        "scope": "unit", "owner": "implementer", "spec_ref": "", "repro": "",
        "observed": "o", "expected": "e",
        "location": {"path": "src/main.rs", "quote": "absent"},
    }
    steps_dir = p.root / ".factory" / "local" / "hooks" / "steps"
    steps_dir.mkdir(parents=True, exist_ok=True)
    mode = steps_dir.stat().st_mode
    os.chmod(steps_dir, mode & ~stat.S_IWUSR & ~stat.S_IWGRP & ~stat.S_IWOTH)
    try:
        code, err, _r = call(p, a_result(p, data={"findings": [finding]}))
        assert code == 10, err
        assert "marker_unwritable" in err, err
        assert stored(p) is None  # nothing stored without the marker
    finally:
        os.chmod(steps_dir, mode)


def test_finding_schemas_stay_open(tmp_path):
    """P2, platform item 37: a schema that closes the finding object makes
    the rule's rewrite unreturnable. No shipped schema closes it without
    declaring ``grounded``."""
    import pathlib

    schemas = pathlib.Path(rule_projects.__file__).parents[2] / (
        "plugins/vsdd-factory/schemas"
    )
    open_matters: list[pathlib.Path] = []
    for path in sorted(schemas.glob("*.json")):
        document = json.loads(path.read_text(encoding="utf-8"))

        def walk(node, root):
            if isinstance(node, dict):
                ref = node.get("$ref", "")
                if ref.endswith("finding.schema.json"):
                    open_matters.append(schemas / "finding.schema.json")
                if node.get("additionalProperties") is False and isinstance(
                    node.get("properties"), dict
                ) and {"key", "title", "severity"} <= set(node["properties"]):
                    # A closed finding object: it must declare grounded.
                    open_matters.append(path)
                    assert "grounded" in node["properties"], path
                for value in node.values():
                    walk(value, root)
            elif isinstance(node, list):
                for item in node:
                    walk(item, root)

        walk(document, document)
    finding_schema = schemas / "finding.schema.json"
    assert finding_schema in open_matters  # the shipped $ref is covered
    finding = json.loads(finding_schema.read_text(encoding="utf-8"))
    assert finding.get("additionalProperties") is not False
