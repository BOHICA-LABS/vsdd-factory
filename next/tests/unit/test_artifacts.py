"""The artifact registry: every shipped row compiles, the reason codes fire,
and validate and path behave (state-core.md 3.5; rules/artifacts.md)."""

from __future__ import annotations

import io
import json
from pathlib import Path

import pytest

from vsdd.cli import dispatch
from vsdd.config import loads
from vsdd.state import artifacts

from ._repo import make_project

FIXTURES = Path(__file__).resolve().parent / "fixtures" / "artifacts"

CONTRACT_REL = ".factory/specs/product/contracts/2.03/BC-2.03.007.md"
STORY_REL = ".factory/stories/E-9/S-9.01.md"
EPIC_REL = ".factory/stories/E-9/epic.md"


def fixture(name: str) -> str:
    return (FIXTURES / name).read_text(encoding="utf-8")


def codes(problems) -> list[str]:
    return [p.code for p in problems]


def test_shipped_registry_compiles_every_row():
    registry = artifacts.registry()
    assert "contract" in registry.kinds and "story" in registry.kinds
    assert "epic" in registry.kinds and "_unknown" not in registry.kinds
    assert registry.unknown_policy == "block"
    assert registry.raw["story-kind"]  # the story-kind rows ride along
    # The pinned rows compile into real matchers.
    assert registry.kinds["contract"].id_re.fullmatch("BC-2.03.007")
    assert registry.kinds["story"].id_re.fullmatch("S-9.01")
    # Design-notation placeholder rows stay out of the match until shipped.
    assert "subsystem" in registry.kinds  # SS-<NN> matches nothing written yet


def test_kind_of_territory():
    assert artifacts.kind_of(CONTRACT_REL) == "contract"
    assert artifacts.kind_of(STORY_REL) == "story"
    assert artifacts.kind_of(EPIC_REL) == "epic"
    assert artifacts.kind_of(".factory/demo/2026-01-01/a/AC-1.tape") == "demo-evidence"
    assert artifacts.kind_of(".factory/factory.toml") is None
    assert artifacts.kind_of(".factory/local/runs/r.json") is None
    assert artifacts.kind_of("src/module.py") is None
    assert artifacts.kind_of(".factory/demo/misc/stray.txt") == "_unknown"


def test_valid_story_contract_epic_pass():
    config = loads("schema = 1\n")
    assert artifacts.check_text(CONTRACT_REL, fixture("valid_contract.md"), config=config) == []
    assert artifacts.check_text(STORY_REL, fixture("valid_story.md"), config=config) == []
    assert artifacts.check_text(EPIC_REL, fixture("valid_epic.md"), config=config) == []


def test_each_reason_code():
    config = loads("schema = 1\n")
    expectations = {
        "invalid_frontmatter.md": (CONTRACT_REL, "frontmatter_invalid"),
        "invalid_reference_malformed.md": (CONTRACT_REL, "reference_malformed"),
        "invalid_heading_missing.md": (STORY_REL, "heading_missing"),
        "invalid_size_cap.md": (STORY_REL, "size_cap"),
        "invalid_long_line.md": (STORY_REL, "size_cap"),
        "invalid_forbidden_pattern.md": (STORY_REL, "forbidden_pattern"),
        "invalid_table_malformed.md": (STORY_REL, "table_malformed"),
        "invalid_criterion_contract.md": (STORY_REL, "criterion_contract"),
        "invalid_id_mismatch.md": (".factory/specs/product/contracts/2.03/BC-2.03.999.md", "id_mismatch"),
    }
    for name, (rel, expected) in expectations.items():
        found = codes(artifacts.check_text(rel, fixture(name), config=config))
        assert expected in found, (name, found)
    # The long-line size_cap names the line number (S3R-state-core-11).
    (found,) = [
        p
        for p in artifacts.check_text(STORY_REL, fixture("invalid_long_line.md"), config=config)
        if p.code == "size_cap" and "line 6" in p.message
    ]
    assert "5,047" in found.message
    # unknown_artifact_path: under an artifact directory, matching no kind.
    found = artifacts.check_text(
        ".factory/demo/misc/stray.txt", fixture("invalid_unknown_path.md"), config=config
    )
    assert "unknown_artifact_path" in codes(found)
    # Outside the artifact directories nothing is checked.
    assert artifacts.check_text("src/module.py", "whatever", config=config) == []


def test_smaller_write_always_allowed():
    config = loads("schema = 1\n")
    big = fixture("invalid_size_cap.md")
    rel = STORY_REL
    assert "size_cap" in codes(artifacts.check_text(rel, big, config=config))
    # A write that makes the file smaller is always allowed.
    old_size = len(big.encode("utf-8")) + 2000
    grown = big + "more\n"
    assert "size_cap" not in codes(
        artifacts.check_text(rel, grown, config=config, old_size=old_size)
    )
    # Growing past the old size is capped again: the old write sat at the
    # same size, the new one is 1,000 bytes heavier.
    assert "size_cap" in codes(
        artifacts.check_text(
            rel, big + "much more\n" * 1000, config=config, old_size=len(big.encode("utf-8"))
        )
    )


def test_dangling_contract_only_with_references(tmp_path):
    config = loads("schema = 1\n")
    # Without the root: syntax only.
    assert "dangling_reference" not in codes(
        artifacts.check_text(STORY_REL, fixture("valid_story.md"), config=config)
    )
    # With the root: the contract does not exist there.
    found = artifacts.check_text(
        STORY_REL, fixture("valid_story.md"), config=config, references=tmp_path
    )
    assert "dangling_reference" in codes(found)
    # Plant the contract: the reference resolves.
    target = tmp_path / CONTRACT_REL
    target.parent.mkdir(parents=True)
    target.write_text(fixture("valid_contract.md"), encoding="utf-8")
    found = artifacts.check_text(
        STORY_REL, fixture("valid_story.md"), config=config, references=tmp_path
    )
    assert "dangling_reference" not in codes(found)


def test_criterion_names_a_listed_contract():
    config = loads("schema = 1\n")
    body = (
        "---\nid: S-9.01\ntitle: T\nstatus: draft\ncycle: C-1\nkind: feature\n"
        "priority: P1\ncriticality: high\ncontracts: [BC-2.03.007]\n---\n\n"
        "## Acceptance criteria\n"
        "### AC-1 (BC-9.99.001)\nThe wrong one.\n"
    )
    found = artifacts.check_text(STORY_REL, body, config=config)
    assert "criterion_contract" in codes(found)
    duplicate = (
        "---\nid: S-9.01\ntitle: T\nstatus: draft\ncycle: C-1\nkind: feature\n"
        "priority: P1\ncriticality: high\ncontracts: [BC-2.03.007]\n---\n\n"
        "## Acceptance criteria\n"
        "### AC-1 (BC-2.03.007)\nOne.\n"
        "### AC-1 (BC-2.03.007)\nTwo.\n"
    )
    found = artifacts.check_text(STORY_REL, duplicate, config=config)
    assert "criterion_contract" in codes(found)


def test_path_for_round_trip(tmp_path):
    assert artifacts.path_for("S-9.01") == ("story", STORY_REL)
    assert artifacts.path_for("BC-2.03.007") == ("contract", CONTRACT_REL)
    assert artifacts.path_for("ZZ-nothing") is None
    # The round trip: a planted contract loads back by its identifier.
    root = _planted_project(tmp_path)
    loaded = artifacts.load_artifact(root, "BC-2.03.007")
    assert (loaded.kind, loaded.id, loaded.rel) == ("contract", "BC-2.03.007", CONTRACT_REL)
    with pytest.raises(artifacts.ArtifactError):
        artifacts.load_artifact(root, "BC-9.99.999")


def _planted_project(tmp_path: Path) -> Path:
    root = tmp_path / "planted"
    target = root / CONTRACT_REL
    target.parent.mkdir(parents=True)
    target.write_text(fixture("valid_contract.md"), encoding="utf-8")
    return root


def test_path_command(tmp_path):
    root = make_project(tmp_path / "p")
    state = root / ".factory"
    (state / "stories" / "E-9").mkdir(parents=True)
    (state / "stories" / "E-9" / "S-9.01.md").write_text(
        fixture("valid_story.md"), encoding="utf-8"
    )
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(
        ["-C", str(root), "path", "S-9.01"], stdout=out, stderr=err, stdin_tty=True
    )
    assert (code, out.getvalue().strip()) == (0, STORY_REL)
    # Under --json the whole object is printed (finding S3R-state-core-29).
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(
        ["-C", str(root), "path", "S-9.01", "--json"], stdout=out, stderr=err, stdin_tty=True
    )
    payload = json.loads(out.getvalue())
    assert (code, payload["ok"]) == (0, True)
    assert (payload["id"], payload["kind"], payload["path"], payload["exists"]) == (
        "S-9.01",
        "story",
        STORY_REL,
        True,
    )
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(
        ["-C", str(root), "path", "S-9.02", "--json"], stdout=out, stderr=err, stdin_tty=True
    )
    payload = json.loads(out.getvalue())
    assert (code, payload["ok"], payload["exists"]) == (1, False, False)
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(
        ["-C", str(root), "path", "ZZ-nope", "--json"], stdout=out, stderr=err, stdin_tty=True
    )
    payload = json.loads(out.getvalue())
    assert (code, payload["ok"], payload["kind"]) == (1, False, "")


def test_validate_exit_codes(tmp_path):
    root = make_project(tmp_path / "v")
    state = root / ".factory"
    (state / "stories" / "E-9").mkdir(parents=True)
    # The contract the valid story names exists: no dangling reference.
    contract = state / "specs" / "product" / "contracts" / "2.03" / "BC-2.03.007.md"
    contract.parent.mkdir(parents=True)
    contract.write_text(fixture("valid_contract.md"), encoding="utf-8")
    (state / "stories" / "E-9" / "S-9.01.md").write_text(
        fixture("valid_story.md"), encoding="utf-8"
    )
    (state / "stories" / "E-9" / "S-9.02.md").write_text(
        fixture("invalid_heading_missing.md"), encoding="utf-8"
    )
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(["-C", str(root), "validate", "--json"], stdout=out, stderr=err, stdin_tty=True)
    payload = json.loads(out.getvalue())
    assert (code, payload["ok"]) == (1, False)
    assert {p["code"] for p in payload["problems"]} >= {"heading_missing"}
    # Only artifact-territory files are counted: .git, factory.toml and
    # notes.json are not (finding S3R-state-core-26).
    assert payload["checked"] == 3
    # One syntax: --changed selects the changed files of the state worktree.
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(
        ["-C", str(root), "validate", "--json", "--changed"], stdout=out, stderr=err, stdin_tty=True
    )
    payload = json.loads(out.getvalue())
    assert code == 1 and payload["problems"]
    # --changed and explicit paths together are a usage error.
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(
        ["-C", str(root), "validate", "--json", "--changed", ".factory/stories/E-9/S-9.01.md"],
        stdout=out, stderr=err, stdin_tty=True,
    )
    assert code == 2
    # Explicit paths validate only those.
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(
        ["-C", str(root), "validate", "--json", ".factory/stories/E-9/S-9.01.md"],
        stdout=out,
        stderr=err,
        stdin_tty=True,
    )
    payload = json.loads(out.getvalue())
    assert (code, payload["ok"], payload["checked"]) == (0, True, 1)
    # A path that does not exist is refused, not silently zero-checked.
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(
        ["-C", str(root), "validate", "--json", ".factory/stories/E-9/S-9.99.md"],
        stdout=out,
        stderr=err,
        stdin_tty=True,
    )
    assert code == 2 and "not a file" in err.getvalue()


def test_validate_reports_unreadable_files(tmp_path):
    """A file that cannot be decoded is an error problem, never skipped
    (finding S3R-state-core-9)."""
    root = make_project(tmp_path / "u")
    state = root / ".factory"
    (state / "stories" / "E-9").mkdir(parents=True)
    (state / "stories" / "E-9" / "S-9.01.md").write_bytes(b"\xff\xfe\x00 not utf-8")
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(["-C", str(root), "validate", "--json"], stdout=out, stderr=err, stdin_tty=True)
    payload = json.loads(out.getvalue())
    assert code == 1
    assert any(p["code"] == "unreadable" for p in payload["problems"])
    # The same through check_paths directly.
    from vsdd.config import loads

    found = artifacts.check_paths(root, config=loads("schema = 1\n"))
    assert "unreadable" in codes(found)


def test_changed_paths(tmp_path):
    root = make_project(tmp_path / "c")
    state = root / ".factory"
    (state / "stories" / "E-9").mkdir(parents=True)
    (state / "stories" / "E-9" / "S-9.01.md").write_text("new\n", encoding="utf-8")
    (state / "factory.toml").write_text(
        (state / "factory.toml").read_text(encoding="utf-8") + "\n# touched\n",
        encoding="utf-8",
    )
    from vsdd.state.artifacts import changed_paths

    changed = changed_paths(root)
    assert any(p.endswith("S-9.01.md") for p in changed)
    assert any(p.endswith("factory.toml") for p in changed)
    # Dotfiles, spaces and non-ASCII names come back whole and prefixed
    # (finding S3R-state-core-10): no lstrip mishap, no C-quoting.
    (state / ".gitkeep").write_text("keep\n", encoding="utf-8")
    (state / "stories" / "E-9" / "my file.md").write_text("spaces\n", encoding="utf-8")
    (state / "stories" / "E-9" / "Störung.md").write_text("unicode\n", encoding="utf-8")
    changed = changed_paths(root)
    assert ".factory/.gitkeep" in changed
    assert ".factory/stories/E-9/my file.md" in changed
    assert ".factory/stories/E-9/Störung.md" in changed


def test_size_cap_waiver_matches_both_path_forms():
    """caps.waived matches the documented form (relative to the state
    directory) as well as the project-relative one (S3R-state-core-21)."""
    config = loads("schema = 1\n")
    body = (
        "---\nid: S-9.01\ntitle: Big\nstatus: draft\ncycle: C-1\nkind: feature\n"
        "priority: P1\ncriticality: high\ncontracts: [BC-2.03.007]\n---\n\n"
        "## Acceptance criteria\n\n### AC-1 (BC-2.03.007)\n\n"
        + "\n".join("x" * 900 for _ in range(19))
        + "\n"
    )
    waived_state_relative = loads(
        'schema = 1\n[caps]\nwaived = ["stories/E-9/S-9.01.md"]\n'
    )
    found = artifacts.check_text(STORY_REL, body, config=waived_state_relative)
    assert "size_cap" not in codes(found)
    waived_project_relative = loads(
        'schema = 1\n[caps]\nwaived = ["' + STORY_REL + '"]\n'
    )
    found = artifacts.check_text(STORY_REL, body, config=waived_project_relative)
    assert "size_cap" not in codes(found)
    # A waiver for another file still leaves the cap to fire.
    other = loads('schema = 1\n[caps]\nwaived = ["stories/E-9/S-9.09.md"]\n')
    assert "size_cap" in codes(artifacts.check_text(STORY_REL, body, config=other))


def test_malformed_criterion_heading_is_flagged():
    """A level-3 heading starting AC- that is not '### AC-N (...)' is a
    criterion_contract, not a silent escape (finding S3R-state-core-22)."""
    config = loads("schema = 1\n")
    malformed = (
        "---\nid: S-9.01\ntitle: T\nstatus: draft\ncycle: C-1\nkind: feature\n"
        "priority: P1\ncriticality: high\ncontracts: [BC-2.03.007]\n---\n\n"
        "## Acceptance criteria\n"
        "### AC-1: Title (BC-2.03.007)\nThe wrong spelling.\n"
        "### AC-2: no parenthesis either\nAnother wrong one.\n"
    )
    found = artifacts.check_text(STORY_REL, malformed, config=config)
    mal = [p for p in found if p.code == "criterion_contract" and "AC-1: Title" in p.message]
    assert len(mal) == 1
    assert any("AC-2" in p.message for p in found if p.code == "criterion_contract")


def test_registry_compiles_cells_loudly(monkeypatch):
    """Bad type cells and dangling ref: targets fail the registry build,
    naming the row (finding S3R-state-core-19)."""
    from vsdd.state.artifacts import RegistryError

    good_row = {
        "glob": ".factory/things/E-*/S-*.md",
        "dir": "E-{1}",
        "id": r"S-\d+\.\d{2}",
        "frontmatter": {"id": "str", "title": "str"},
    }

    def patched_registry(row):
        monkeypatch.setattr(
            artifacts, "load_policy", lambda name: {"things": row}
        )
        artifacts.registry.cache_clear()
        try:
            return artifacts.registry()
        finally:
            artifacts.registry.cache_clear()

    try:
        # Unknown type.
        bad = dict(good_row, frontmatter={"title": "banana"})
        with pytest.raises(RegistryError) as raised:
            patched_registry(bad)
        assert "[things]" in str(raised.value) and "banana" in str(raised.value)
        # A ref: to a kind with neither an id row nor a constant pattern.
        dangling = dict(good_row, frontmatter={"who": "ref:nohope"})
        with pytest.raises(RegistryError) as raised:
            patched_registry(dangling)
        assert "nohope" in str(raised.value)
        # The error names the row and key holding the bad cell, not only the
        # target (finding S3R-state-core-C1-7).
        assert "[things].who" in str(raised.value)
        # A list member type is validated too, and a good row compiles.
        with pytest.raises(RegistryError):
            patched_registry(dict(good_row, frontmatter={"tags": "list[banana]"}))
        compiled = patched_registry(good_row)
        assert "things" in compiled.kinds
    finally:
        # Undo the patched loader before rebuilding the cached registry, so
        # the real one is what the later tests read.
        monkeypatch.undo()
        artifacts.registry.cache_clear()
        artifacts.registry()
