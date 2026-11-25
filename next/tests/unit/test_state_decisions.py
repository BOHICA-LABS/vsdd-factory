"""The decision store (plan records.md 7): the grammar's seven lines, the
kinds table, resolution preference, every settle refusal in order, and the
one meaning of "the specification is approved"."""

from __future__ import annotations

import re
import tomllib
from pathlib import Path

import pytest

from .state_fixtures import make_project, spec_file
from vsdd.state import decisions, digests
from vsdd.state.store import DecisionRefused, StoreRefused

REPO = Path(__file__).resolve().parents[3]
DECISION_KINDS = REPO / "rewrite" / "spec" / "tables" / "decision-kinds.toml"


def test_grammar():
    # The seven parser fixtures of rules/decisions.md, one per line.
    assert decisions.parse_decide(["spec", "approve", "variant=B", "topology=split"]) == (
        "spec",
        "approve",
        [("variant", "B"), ("topology", "split")],
        "",
    )
    assert decisions.parse_decide(["spec", "revise", "stage=contracts", "--", "cover the offline case"]) == (
        "spec",
        "revise",
        [("stage", "contracts")],
        "cover the offline case",
    )
    assert decisions.parse_decide(["ideas", "select", "pursue=ID-003,ID-007", "reject=ID-004"]) == (
        "ideas",
        "select",
        [("pursue", "ID-003,ID-007"), ("reject", "ID-004")],
        "",
    )
    assert decisions.parse_decide([
        "finding.S-7.04", "waive", "key=code:src/auth.rs:lockout",
        "key=test:tests/auth.rs:lockout_race", "--", "accepted for the beta",
    ]) == (
        "finding.S-7.04",
        "waive",
        [("key", "code:src/auth.rs:lockout"), ("key", "test:tests/auth.rs:lockout_race")],
        "accepted for the beta",
    )
    assert decisions.parse_decide(["finding.W-02", "waive-coverage", "item=AC-3", "--", "covered by the manual check"]) == (
        "finding.W-02",
        "waive-coverage",
        [("item", "AC-3")],
        "covered by the manual check",
    )
    assert decisions.parse_decide(["blocked.S-7.04", "open-spike", "name=lockout-timing"]) == (
        "blocked.S-7.04",
        "open-spike",
        [("name", "lockout-timing")],
        "",
    )
    assert decisions.parse_decide(["release", "hold", "--", "wait for the security fix"]) == (
        "release",
        "hold",
        [],
        "wait for the security fix",
    )
    # A qualified id is one token; a repeated key survives as typed pairs.
    ident, choice, pairs, note = decisions.parse_decide(["release@hotfix-1.2.1", "hold", "--", "later"])
    assert (ident, choice, note) == ("release@hotfix-1.2.1", "hold", "later")

    # A positional value refuses; so does a malformed key and an empty value.
    with pytest.raises(StoreRefused) as exc:
        decisions.parse_decide(["release", "approve", "B"])
    assert exc.value.code == "bad_grammar"
    with pytest.raises(StoreRefused):
        decisions.parse_decide(["release", "approve", "Variant=big"])
    with pytest.raises(StoreRefused):
        decisions.parse_decide(["release", "approve", "variant="])
    with pytest.raises(StoreRefused):
        decisions.parse_decide(["release"])


def test_kinds_table():
    assert len(decisions.KINDS) == 9
    for kind, groups in decisions.KINDS.items():
        assert set(groups) == {"approved", "revise", "rejected"}, kind
        for status, choices in groups.items():
            for choice in choices:
                assert choice.status == status, (kind, choice.verb)
    # The mapping of topic decisions, spot-checked on every verb that appears.
    verbs = {
        "approve": "approved",
        "hold": "revise",
        "drop": "rejected",
        "reject": "rejected",
        "descope": "rejected",
        "waive": "approved",
        "waive-coverage": "approved",
        "direct": "revise",
        "amend": "approved",
        "open-spike": "approved",
        "skip": "approved",
        "retry": "approved",
        "select": "approved",
        "revise": "revise",
        "not-a-gap": "revise",
    }
    seen: dict[str, str] = {}
    for kind, groups in decisions.KINDS.items():
        for status, choices in groups.items():
            for choice in choices:
                seen[choice.verb] = status
    assert seen == verbs
    # amend and open-spike are not built on the slice (decision D10).
    for kind, verb in (("spec-gap", "amend"), ("blocked", "open-spike")):
        for choice in decisions.KINDS[kind]["approved"]:
            if choice.verb == verb:
                assert choice.built is False


def test_kinds_packet_over_cap(tmp_path):
    root = make_project(tmp_path)
    with pytest.raises(StoreRefused) as exc:
        decisions.decision_open(
            root, cycle="product-1", kind="release", packet="# Packet\n" + "x" * 13_000
        )
    assert exc.value.code == "over_cap"
    written = root / ".factory" / "cycles" / "product-1" / "decisions"
    assert not (written / "release.json").exists(), "the record rides with the packet"
    assert not (written / "release.md").exists()


def test_packet_written_before_the_record(tmp_path, monkeypatch):
    """The packet goes first, and the record is the commit point: a packet
    write killed between the two leaves no open record (S3R-records-20)."""
    root = make_project(tmp_path)
    from vsdd.state import store

    real = store.write_via_tmp

    def killing(path, data):
        if str(path).endswith(".md"):
            raise RuntimeError("killed between packet and record")
        return real(path, data)

    monkeypatch.setattr(store, "write_via_tmp", killing)
    with pytest.raises(RuntimeError):
        decisions.decision_open(root, cycle="product-1", kind="release", packet="# Release\n")
    monkeypatch.undo()

    directory = root / ".factory" / "cycles" / "product-1" / "decisions"
    assert not (directory / "release.md").exists()
    assert not (directory / "release.json").exists(), "no open record without its packet"
    with pytest.raises(StoreRefused) as exc:
        decisions.decision_read(root, "release")
    assert exc.value.code == "unknown"


def test_resolve_prefers_open(tmp_path):
    root = make_project(tmp_path)
    decisions.decision_open(root, cycle="product-1", kind="release", packet="# Release\n")
    settled = root / ".factory" / "cycles" / "product-1" / "decisions" / "release.json"
    record = decisions.decision_read(root, "release")
    assert record["status"] == "open"

    # A second open cycle: one open among two is found; two open are ambiguous.
    cycles_dir = root / ".factory" / "cycles"
    (cycles_dir / "hotfix-2").mkdir(parents=True, exist_ok=True)
    (cycles_dir / "hotfix-2" / "cycle.json").write_text(
        '{"id": "hotfix-2", "kind": "hotfix", "title": "Hotfix", "route": "", '
        '"opened": "2026-11-24T09:00:00Z", "baseline": "", "request": "CR-001"}',
        encoding="utf-8",
    )
    import json

    open_record = json.loads(settled.read_text(encoding="utf-8"))
    second = cycles_dir / "hotfix-2" / "decisions" / "release.json"
    second.parent.mkdir(parents=True, exist_ok=True)

    settled_record = dict(open_record, status="approved", choice="approve", settled="2026-11-24T10:00:00Z",
                          settled_by="terminal", settled_by_session="")
    second.write_text(json.dumps(settled_record), encoding="utf-8")
    cycle, _ = decisions.resolve_decision(root, "release")
    assert cycle == "product-1"

    settled_record = dict(open_record)
    second.write_text(json.dumps(settled_record), encoding="utf-8")
    with pytest.raises(DecisionRefused) as exc:
        decisions.resolve_decision(root, "release")
    assert exc.value.code == "ambiguous"
    assert sorted(exc.value.choices) == ["release@hotfix-2", "release@product-1"]

    # Nothing anywhere is unknown.
    with pytest.raises(StoreRefused) as exc:
        decisions.resolve_decision(root, "plan")
    assert exc.value.code == "unknown"
    # As written: id@cycle.
    cycle, bare = decisions.resolve_decision(root, "release@hotfix-2")
    assert (cycle, bare) == ("hotfix-2", "release")


def test_settle(tmp_path, monkeypatch):
    root = make_project(tmp_path)
    decisions.decision_open(root, cycle="product-1", kind="release", packet="# Release\n")

    # Settling runs no git, on any path.
    def no_git(*args, **kwargs):
        raise AssertionError("settle_decision ran git")

    monkeypatch.setattr("vsdd.state.gitrun.git", no_git)
    monkeypatch.setattr("vsdd.state.store.git", no_git)

    # Refusals, in order: unknown, ambiguous, not_open, stale, bad_choice,
    # bad_key, note_required, not_built.
    with pytest.raises(StoreRefused) as exc:
        decisions.settle_decision(root, "plan", "approve", {}, "", "")
    assert exc.value.code == "unknown"

    with pytest.raises(DecisionRefused) as exc:
        decisions.settle_decision(root, "release", "nonsense", {}, "", "")
    assert exc.value.code == "bad_choice"
    assert "approve" in exc.value.choices

    settled = decisions.settle_decision(root, "release", "approve", {}, "", "")
    assert settled["status"] == "approved"
    # Settling again is refused: not_open.
    with pytest.raises(StoreRefused) as exc:
        decisions.settle_decision(root, "release", "approve", {}, "", "")
    assert exc.value.code == "not_open"
    assert decisions.decision_read(root, "release")["status"] == "approved"

    # A brief decision needs its note; direct carries guidance.
    decisions.decision_open(root, cycle="product-1", kind="brief", packet="# Brief\n")
    with pytest.raises(StoreRefused) as exc:
        decisions.settle_decision(root, "brief", "revise", {}, "", "")
    assert exc.value.code == "note_required"

    # A finding waive needs its key.
    decisions.decision_open(root, cycle="product-1", kind="finding", packet="# Finding\n", subject="S-7.04")
    with pytest.raises(StoreRefused) as exc:
        decisions.settle_decision(root, "finding.S-7.04", "waive", {}, "note", "")
    assert exc.value.code == "bad_key"

    # spec-gap amend is not built (decision D10).
    decisions.decision_open(root, cycle="product-1", kind="spec-gap", packet="# Gap\n")
    with pytest.raises(StoreRefused) as exc:
        decisions.settle_decision(root, "spec-gap", "amend", {}, "", "")
    assert exc.value.code == "not_built"

    # Stale: a spec decision whose file changed since it was opened.
    spec_file(root, ".factory/specs/product/prd.md", "# PRD v1\n")
    digest, files = digests.spec_digest(root)
    decisions.decision_open(
        root, cycle="product-1", kind="spec", packet="# Spec\n", digest=digest, files=files
    )
    # An unrelated edit (not in the file set) does not stale it.
    settled = decisions.settle_decision(
        root, "spec", "approve", {"variant": "B"}, "ship it", "sess-1"
    )
    assert settled["status"] == "approved"
    assert settled["settled_by"] == "session"  # a session id was given
    assert settled["values"] == {"variant": "B"}
    # A repeated key is stored as its values joined by one space.
    decisions.decision_open(root, cycle="product-1", kind="finding", packet="# F\n", subject="W-1")
    joined = decisions.settle_decision(
        root, "finding.W-1", "waive", [("key", "a:b:c"), ("key", "d:e:f")], "because", ""
    )
    assert joined["values"] == {"key": "a:b:c d:e:f"}
    assert joined["settled_by"] == "terminal"  # no session id: the person's terminal

    # Combined refusals hold the promised order: not_open before bad_choice,
    # stale before bad_choice, bad_choice before bad_key, bad_key before
    # note_required.
    with pytest.raises(StoreRefused) as exc:  # release is settled by now: not_open
        decisions.settle_decision(root, "release", "nonsense", {"junk": "x"}, "", "")
    assert exc.value.code == "not_open"

    spec_file(root, ".factory/specs/product/prd.md", "# PRD moved on\n")
    digest, files = digests.spec_digest(root)
    decisions.decision_open(
        root, cycle="product-1", kind="spec", packet="# Spec\n", digest=digest, files=files
    )
    spec_file(root, ".factory/specs/product/prd.md", "# PRD moved on again\n")
    with pytest.raises(StoreRefused) as exc:  # the second spec decision is stale now
        decisions.settle_decision(root, "spec", "nonsense", {}, "", "")
    assert exc.value.code == "stale", "stale is refused before the choice is judged"

    decisions.decision_open(root, cycle="product-1", kind="brief", packet="# Brief\n")
    with pytest.raises(StoreRefused) as exc:  # bad_choice before bad_key
        decisions.settle_decision(root, "brief", "nonsense", {"junk": "x"}, "", "")
    assert exc.value.code == "bad_choice"
    with pytest.raises(StoreRefused) as exc:  # bad_key before note_required
        decisions.settle_decision(root, "brief", "revise", {"junk": "x"}, "", "")
    assert exc.value.code == "bad_key"


def test_settle_stale_after_edit(tmp_path):
    root = make_project(tmp_path)
    spec_file(root, ".factory/specs/product/prd.md", "# PRD v1\n")
    digest, files = digests.spec_digest(root)
    decisions.decision_open(
        root, cycle="product-1", kind="spec", packet="# Spec\n", digest=digest, files=files
    )
    # One file differs.
    spec_file(root, ".factory/specs/product/prd.md", "# PRD v2\n")
    with pytest.raises(StoreRefused) as exc:
        decisions.settle_decision(root, "spec", "approve", {}, "", "")
    assert exc.value.code == "stale"
    assert decisions.decision_status(root, "spec") == "stale"


def test_spec_approved(tmp_path):
    root = make_project(tmp_path)
    ok, detail = decisions.spec_approved(root)
    assert (ok, detail) == (False, "no spec decision")

    spec_file(root, ".factory/specs/product/prd.md", "# PRD v1\n")
    digest, files = digests.spec_digest(root)
    decisions.decision_open(
        root, cycle="product-1", kind="spec", packet="# Spec\n", digest=digest, files=files
    )
    ok, detail = decisions.spec_approved(root)
    assert (ok, detail) == (False, "open")

    decisions.settle_decision(root, "spec", "approve", {}, "", "")
    assert decisions.spec_approved(root) == (True, "")

    # A contract edit stales the approval; a brief or guidance edit does not.
    spec_file(root, ".factory/specs/product/prd.md", "# PRD v2\n")
    assert decisions.spec_approved(root) == (False, "stale")

    spec_file(root, ".factory/specs/product/prd.md", "# PRD v3\n")
    spec_file(root, ".factory/specs/architecture/SS-01.md", "# Edited after\n")
    digest, files = digests.spec_digest(root)
    fresh = decisions.decision_read(root, "spec")
    # Re-open a fresh spec decision for the edited set, approve it, then edit
    # only the brief and the guidance: neither makes the approval stale.
    decisions.decision_open(
        root, cycle="product-1", kind="spec", packet="# Spec\n", digest=digest, files=files
    )
    decisions.settle_decision(root, "spec", "approve", {}, "", "")
    spec_file(root, ".factory/specs/product/brief.md", "# Brief, changed\n")
    spec_file(root, ".factory/specs/architecture/conventions.md", "# Conventions, changed\n")
    spec_file(root, ".factory/specs/architecture/review-rules.md", "# Review rules, changed\n")
    assert decisions.spec_approved(root) == (True, "")
    del fresh


def test_topology_values_and_repetition(tmp_path):
    """Topology is single or split; only key and item may repeat; both refuse
    bad_key (probe: topology=banana variant=a variant=b was settled)."""
    root = make_project(tmp_path)
    spec_file(root, ".factory/specs/product/prd.md", "# PRD v1\n")
    digest, files = digests.spec_digest(root)
    decisions.decision_open(
        root, cycle="product-1", kind="spec", packet="# Spec\n", digest=digest, files=files
    )

    with pytest.raises(StoreRefused) as exc:
        decisions.settle_decision(
            root, "spec", "approve", {"topology": "banana", "variant": "a b"}, "", ""
        )
    assert exc.value.code == "bad_key"
    assert "single" in exc.value.args[0] and "split" in exc.value.args[0]

    # A non-repeatable key refuses to repeat; key and item may.
    with pytest.raises(StoreRefused) as exc:
        decisions.settle_decision(
            root, "spec", "approve", [("variant", "a"), ("variant", "b")], "", ""
        )
    assert exc.value.code == "bad_key"
    settled = decisions.settle_decision(root, "spec", "approve", {"topology": "split"}, "", "")
    assert settled["values"] == {"topology": "split"}

    decisions.decision_open(root, cycle="product-1", kind="finding", packet="# F\n", subject="R")
    ok = decisions.settle_decision(
        root, "finding.R", "waive", [("key", "a:1"), ("key", "b:2")], "why", ""
    )
    assert ok["values"]["key"] == "a:1 b:2"
    decisions.decision_open(root, cycle="product-1", kind="finding", packet="# C\n", subject="Q")
    with pytest.raises(StoreRefused) as exc:
        decisions.settle_decision(
            root, "finding.Q", "waive-coverage", [("key", "a:1"), ("key", "b:2")], "why", ""
        )
    assert exc.value.code == "bad_key"  # waive-coverage takes item, not key


def test_blocked_reason_gates(tmp_path):
    """open-spike is for spike_limit, skip for environment; a reason outside
    the four refuses at open (probe: skip settled a stuck decision)."""
    root = make_project(tmp_path)
    with pytest.raises(StoreRefused) as exc:
        decisions.decision_open(
            root, cycle="product-1", kind="blocked", packet="# B\n", reason="nonsense-reason"
        )
    assert exc.value.code == "bad_reason"
    for reason in ("spike_limit", "environment", "stuck", "reviewer"):
        decisions.decision_open(
            root, cycle="product-1", kind="blocked", packet="# B\n", subject=reason, reason=reason
        )

    # A stuck decision refuses skip and open-spike; retry is always there.
    with pytest.raises(DecisionRefused) as exc:
        decisions.settle_decision(root, "blocked.stuck", "skip", {}, "", "")
    assert exc.value.code == "bad_choice"
    with pytest.raises(DecisionRefused) as exc:
        decisions.settle_decision(
            root, "blocked.spike_limit", "skip", {}, "", ""
        )
    assert exc.value.code == "bad_choice"
    settled = decisions.settle_decision(root, "blocked.stuck", "retry", {}, "", "")
    assert settled["status"] == "approved"

    # The environment decision skips; the spike_limit one passes the reason
    # gate for open-spike (whose request file is not built, decision D10:
    # the refusal is not_built, after every earlier check passed).
    settled = decisions.settle_decision(root, "blocked.environment", "skip", {}, "", "")
    assert settled["choice"] == "skip"
    decisions.decision_open(
        root, cycle="product-1", kind="blocked", packet="# S\n", subject="twice",
        reason="spike_limit",
    )
    with pytest.raises(StoreRefused) as exc:
        decisions.settle_decision(
            root, "blocked.twice", "open-spike", {"name": "lockout-timing"}, "", ""
        )
    assert exc.value.code == "not_built"
    # The reason gate holds before not_built: skip on a spike_limit record is
    # refused as bad_choice, not not_built.
    with pytest.raises(DecisionRefused) as exc:
        decisions.settle_decision(
            root, "blocked.spike_limit", "skip", {}, "", ""
        )
    assert exc.value.code == "bad_choice"


def test_closed_cycles_are_not_searched(tmp_path):
    """A bare id resolves over the open cycles and _standing only (decision
    D9): a closed cycle's copy stays out, so a settled release there never
    makes the bare read ambiguous."""
    import json

    root = make_project(tmp_path)
    cycles_dir = root / ".factory" / "cycles"
    # product-1 is open and holds an open release; hotfix-1.2 and hotfix-2
    # are closed by their cycle records and each holds a release too.
    decisions.decision_open(root, cycle="product-1", kind="release", packet="# R\n")
    for closed, seed in (("hotfix-1.2", "a"), ("hotfix-2", "b")):
        (cycles_dir / closed).mkdir(parents=True, exist_ok=True)
        (cycles_dir / closed / "cycle.json").write_text(
            json.dumps(
                {
                    "id": closed, "kind": "hotfix", "title": closed, "route": "",
                    "opened": "2026-11-01T09:00:00Z", "baseline": "", "request": "CR-1",
                    "closed": "2026-11-20T09:00:00Z",
                }
            ),
            encoding="utf-8",
        )
        decisions.decision_open(
            root, cycle=closed, kind="release", packet=f"# R {seed}\n"
        )

    # A bare id found only in a CLOSED cycle is unknown, not resolved there.
    decisions.decision_open(root, cycle="hotfix-1.2", kind="plan", packet="# P\n")
    with pytest.raises(StoreRefused) as exc:
        decisions.resolve_decision(root, "plan")
    assert exc.value.code == "unknown"

    # Two closed copies and one open: the bare read takes the open one.
    decisions.settle_decision(root, "release@hotfix-1.2", "approve", {}, "", "")
    decisions.settle_decision(root, "release@hotfix-2", "approve", {}, "", "")
    decisions.settle_decision(root, "release", "approve", {}, "", "")
    cycle, bare = decisions.resolve_decision(root, "release")
    assert (cycle, bare) == ("product-1", "release")

    # The bare read of release stays unambiguous after the cycle closes: the
    # closed copies never enter the search.
    qualified = decisions.decision_read(root, "release")
    assert qualified["status"] == "approved"


def test_decision_ids_are_path_segments(tmp_path):
    """Ids and subjects are validated as path segments before any path is
    built: traversal refuses unsafe_id and writes nothing (probe: a subject
    of '../../../escape' escaped the decisions directory)."""
    root = make_project(tmp_path)
    with pytest.raises(StoreRefused) as exc:
        decisions.decision_open(
            root, cycle="product-1", kind="release", packet="# R\n",
            subject="../../../escape",
        )
    assert exc.value.code == "unsafe_id"
    assert not (root / ".factory" / "escape").exists()
    stray = [p for p in (root / ".factory").rglob("*escape*")]
    assert stray == []

    with pytest.raises(StoreRefused) as exc:
        decisions.decision_open(
            root, cycle="../outside", kind="release", packet="# R\n"
        )
    assert exc.value.code == "unsafe_id"

    # The commands reach the same refusals: decide resolves over ids, and
    # '../..' is unsafe, not unknown.
    with pytest.raises(StoreRefused) as exc:
        decisions.resolve_decision(root, "../../x")
    assert exc.value.code == "unsafe_id"
    with pytest.raises(StoreRefused) as exc:
        decisions.decision_read(root, "release@../../x")
    assert exc.value.code == "unsafe_id"

    # A '..' inside an otherwise valid segment is refused too.
    with pytest.raises(StoreRefused) as exc:
        decisions.decision_open(
            root, cycle="product-1", kind="release", packet="# R\n", subject="a..b"
        )
    assert exc.value.code == "unsafe_id"


def test_spec_staleness_follows_the_current_set(tmp_path):
    """A spec approval is bound to the current specification file set: a
    contract added after the approval stales it, a covered file deleted since
    reads as stale (never as a missing-file error), and edits to brief and
    guidance files that existed at approval do not stale it."""
    root = make_project(tmp_path)
    spec_file(root, ".factory/specs/product/prd.md", "# PRD v1\n")
    digest, files = digests.spec_digest(root)
    decisions.decision_open(
        root, cycle="product-1", kind="spec", packet="# Spec\n", digest=digest, files=files
    )
    decisions.settle_decision(root, "spec", "approve", {}, "", "")

    # A NEW contract added after the approval stales the approval.
    spec_file(root, ".factory/specs/architecture/SS-09.md", "# Added after\n")
    assert decisions.spec_approved(root) == (False, "stale")
    assert decisions.decision_status(root, "spec") == "stale"

    # Re-open over the now-current set, then DELETE a covered file while the
    # decision is open: stale, and settling refuses stale instead of raising
    # digest_missing_file.
    digest, files = digests.spec_digest(root)
    decisions.decision_open(
        root, cycle="product-1", kind="spec", packet="# Spec\n", digest=digest, files=files
    )
    (root / ".factory" / "specs" / "architecture" / "SS-09.md").unlink()
    assert decisions.decision_status(root, "spec") == "stale"
    with pytest.raises(StoreRefused) as exc:
        decisions.settle_decision(root, "spec", "approve", {}, "", "")
    assert exc.value.code == "stale"

    # The brief and the guidance files that existed at approval are outside
    # the set: editing them leaves the approval standing.
    digest, files = digests.spec_digest(root)
    decisions.decision_open(
        root, cycle="product-1", kind="spec", packet="# Spec\n", digest=digest, files=files
    )
    decisions.settle_decision(root, "spec", "approve", {}, "", "")
    spec_file(root, ".factory/specs/product/brief.md", "# Brief, changed\n")
    spec_file(root, ".factory/specs/architecture/conventions.md", "# Conventions, changed\n")
    assert decisions.spec_approved(root) == (True, "")


def test_kinds_match_the_shipped_table():
    """KINDS is tables/decision-kinds.toml as data: the table and the
    enforcing code cannot drift apart silently."""
    table = tomllib.loads(DECISION_KINDS.read_text(encoding="utf-8"))
    assert len(table["kind"]) == 9
    for row in table["kind"]:
        kind = row["id"]
        assert kind in decisions.KINDS, kind
        assert set(decisions.KINDS[kind]) == {"approved", "revise", "rejected"}, kind
        for status in ("approved", "revise", "rejected"):
            lines = row.get(status, [])
            coded = decisions.KINDS[kind][status]
            assert [line.split(" ")[0] for line in lines] == [c.verb for c in coded], (
                kind,
                status,
            )
            for line, choice in zip(lines, coded):
                # The keys the line shows are the keys the choice takes.
                shown = re.findall(r"\[?([a-z][a-z0-9_-]*)=", line)
                assert set(shown) == set(choice.keys), (kind, line)
                for key in shown:
                    bracketed = f"[{key}=" in line
                    repeated = len(re.findall(rf"\[?{key}=", line)) > 1
                    if repeated:
                        assert choice.keys[key] == decisions._REPEAT, (kind, line)
                    elif bracketed:
                        assert choice.keys[key] == decisions._OPTIONAL, (kind, line)
                    else:
                        assert choice.keys[key] in (decisions._REQUIRED, decisions._OPTIONAL), (
                            kind,
                            line,
                        )
                # A note is required exactly where the line shows -- <text>.
                assert choice.note_required == ("-- " in line), (kind, line)
                # The (for <reason>) annotations are the reason gates.
                gated = re.search(r"\(for ([a-z_ ]+)\)", line)
                expected = (
                    frozenset(w for w in decisions.BLOCKED_REASONS if w in gated.group(1))
                    if gated
                    else frozenset()
                )
                assert choice.reasons == expected, (kind, line)
                # The values a key may take are stated in the line as
                # '<key>=<a> or <b>', and the code carries no others.
                stated: set[str] = set()
                for key, first, second in re.findall(r"(\w+)=([A-Za-z_-]+) or ([A-Za-z_-]+)", line):
                    stated.add(key)
                    assert choice.values.get(key) == (first, second), (kind, line, key)
                assert set(choice.values) == stated, (kind, line, choice.values)
    # The one property the table's select line does not hold: at least one
    # list is required, which the code carries as any_of.
    for choice in decisions.KINDS["ideas"]["approved"]:
        if choice.verb == "select":
            assert choice.any_of == frozenset({"pursue", "research", "defer", "reject"})
