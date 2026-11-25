"""The cycle and closure stores (plan records.md 7, test_closures): creates,
keeps stage order, sets delivery.base; over cap and two cycles refuse. The
note store reads init's ``{"notes": []}`` as no notes (step-4 request)."""

from __future__ import annotations

import pytest

from .state_fixtures import make_project
from vsdd.state import cycles
from vsdd.state.store import DecisionRefused, StoreRefused


def test_closures(tmp_path):
    root = make_project(tmp_path)
    order: list[str] = []

    def mutate(record):
        record["stages"]["draft"] = {"at": "2026-11-24T09:00:00Z"}
        record["stages"]["review"] = {"at": "2026-11-24T10:00:00Z", "rounds": 1}
        record["delivery"] = {"base": "abc1234567890", "branch": "story/S-9.01"}
        order.extend(record["stages"].keys())

    stored = cycles.closure_update(root, "product-1", "story", "S-9.01", mutate)
    assert stored["state"] == "open"
    assert stored["delivery"]["base"] == "abc1234567890"
    # The stage order is data, and the tracked write keeps key order.
    text = (root / ".factory" / "cycles" / "product-1" / "units" / "story.S-9.01.json").read_text(
        encoding="utf-8"
    )
    assert text.index('"draft"') < text.index('"review"')

    found = cycles.closure_read(root, "product-1", "story", "S-9.01")
    assert found["cycle"] == "product-1"
    assert cycles.closure_find(root, "story", "S-9.01")["delivery"]["base"] == "abc1234567890"
    assert cycles.closure_find(root, "story", "S-absent") is None
    assert len(cycles.closures_list(root, "product-1")) == 1

    # A second open cycle holding the same unit is ambiguous.
    cycles.cycle_write(
        root,
        {
            "id": "hotfix-2",
            "kind": "hotfix",
            "title": "Hotfix",
            "route": "",
            "opened": "2026-11-24T09:00:00Z",
            "baseline": "",
            "request": "CR-001",
        },
    )
    cycles.closure_update(root, "hotfix-2", "story", "S-9.01", lambda record: None)
    with pytest.raises(DecisionRefused) as exc:
        cycles.closure_find(root, "story", "S-9.01")
    assert exc.value.code == "ambiguous_unit"

    # Over the cap: a closure record is capped at 16,000 bytes.
    def fat(record):
        record["delivery"] = {"detail": "x" * 20_000}

    with pytest.raises(StoreRefused) as exc:
        cycles.closure_update(root, "hotfix-2", "verify", "", fat)
    assert exc.value.code == "over_cap"

    # A cycle record is capped at 2,000 bytes and refuses bad kinds.
    with pytest.raises(StoreRefused) as exc:
        cycles.cycle_write(
            root,
            {
                "id": "bloated",
                "kind": "product",
                "title": "x" * 3_000,
                "route": "",
                "opened": "2026-11-24T09:00:00Z",
                "baseline": "",
                "request": "brief",
            },
        )
    assert exc.value.code == "over_cap"


def test_two_new_closures_are_independent(tmp_path):
    """Two fresh closures in one process share nothing: a mutation that adds a
    stage to the first must not appear in the second (the shallow-template
    probe), whatever order they are opened in."""
    root = make_project(tmp_path)

    def add_draft(record):
        record["stages"]["draft"] = {"at": "2026-11-24T09:00:00Z"}

    cycles.closure_update(root, "product-1", "story", "S-1.01", add_draft)
    fresh = cycles.closure_update(root, "product-1", "verify", "", lambda record: None)
    assert fresh["stages"] == {}, "a new closure starts with empty stages"
    assert cycles.closure_read(root, "product-1", "verify", "")["stages"] == {}

    # And again, the other way round: the template never carries a stage.
    another = cycles.closure_update(root, "product-1", "story", "S-2", lambda record: None)
    assert another["stages"] == {}
    first = cycles.closure_read(root, "product-1", "story", "S-1.01")
    assert set(first["stages"]) == {"draft"}


def test_notes_empty_form(tmp_path):
    """Init writes ``{"notes": []}``, and that form reads as no notes."""
    root = make_project(tmp_path)
    assert (root / ".factory" / "notes.json").is_file()
    assert cycles.read_notes(root) == []


def test_notes_entries_as_stored(tmp_path):
    """The note store returns entries as they are stored: the ``{at, text}``
    objects the status report shows are not dropped, and a string note reads
    back as the string it is."""
    root = make_project(tmp_path)
    import json

    path = root / ".factory" / "notes.json"
    entries = [
        {"at": "2026-11-24T09:00:00Z", "text": "the spike answered the routing question"},
        "a plain first note",
        {"at": "2026-11-24T10:00:00Z", "text": "reviewer asked for the benchmark"},
    ]
    path.write_text(json.dumps({"notes": entries}), encoding="utf-8")
    assert cycles.read_notes(root) == entries
