"""C1: every shipped result schema carries fixtures, and every fixture passes
or fails the shipped validator exactly as its name and sidecar say (blueprint
4.10, 4.14).

The schemas in ``<plugin>/schemas/*.json`` are the single source; this test is
the harness that keeps the files, the checker and the fixtures in one story.
"""

import json
from pathlib import Path

import pytest

from vsdd.schema import load_registry, validate

NEXT = Path(__file__).resolve().parents[2]
SCHEMAS = NEXT / "plugins" / "vsdd-factory" / "schemas"
FIXTURES = Path(__file__).resolve().parent / "fixtures" / "schemas"

REGISTRY = load_registry(SCHEMAS)


def schema_dirs() -> list[Path]:
    return sorted(p for p in FIXTURES.iterdir() if p.is_dir())


@pytest.fixture(params=[d.name for d in schema_dirs()], ids=[d.name for d in schema_dirs()])
def schema_name(request) -> str:
    return request.param


def schema_of(name: str) -> dict:
    path = SCHEMAS / f"{name}.schema.json"
    assert path.is_file(), f"fixture directory {name} has no matching schemas/{name}.schema.json"
    return REGISTRY[f"{name}.schema.json"]


def fixtures_of(name: str, prefix: str) -> list[Path]:
    directory = FIXTURES / name
    return sorted(p for p in directory.glob(f"{prefix}_*.json") if p.suffix == ".json")


def test_every_shipped_schema_has_a_fixture_directory():
    for schema_path in sorted(SCHEMAS.glob("*.schema.json")):
        stem = schema_path.name[: -len(".schema.json")]
        assert (FIXTURES / stem).is_dir(), f"no fixtures for {schema_path.name}"
        assert fixtures_of(stem, "valid"), f"no valid fixture for {schema_path.name}"
        assert fixtures_of(stem, "invalid"), f"no invalid fixture for {schema_path.name}"


def test_registry_holds_exactly_the_shipped_schemas():
    shipped = {p.name for p in SCHEMAS.glob("*.schema.json")}
    assert set(REGISTRY) == shipped


def test_fixture_directories_match_shipped_schemas():
    stems = {p.name[: -len(".schema.json")] for p in SCHEMAS.glob("*.schema.json")}
    assert {d.name for d in schema_dirs()} == stems


VALID_CASES = [(d.name, p) for d in schema_dirs() for p in fixtures_of(d.name, "valid")]


@pytest.mark.parametrize(
    "name, path", VALID_CASES, ids=[f"{n}/{p.name}" for n, p in VALID_CASES]
)
def test_valid_fixture_passes(name: str, path: Path):
    instance = json.loads(path.read_text(encoding="utf-8"))
    errors = validate(instance, schema_of(name), REGISTRY)
    assert not errors, f"{path.name}: " + "; ".join(str(e) for e in errors)


INVALID_CASES = [(d.name, p) for d in schema_dirs() for p in fixtures_of(d.name, "invalid")]


@pytest.mark.parametrize(
    "name, path", INVALID_CASES, ids=[f"{n}/{p.name}" for n, p in INVALID_CASES]
)
def test_invalid_fixture_fails(name: str, path: Path):
    instance = json.loads(path.read_text(encoding="utf-8"))
    errors = validate(instance, schema_of(name), REGISTRY)
    assert errors, f"{path.name} must not pass the schema"
    # Sidecar naming: invalid_X.expect.txt sits beside invalid_X.json and is
    # required, so the failure is always for the intended reason.
    sidecar = path.parent / (path.stem + ".expect.txt")
    assert sidecar.is_file(), f"missing sidecar {sidecar.name} beside {path.name}"
    expected = sidecar.read_text(encoding="utf-8").strip()
    rendered = "\n".join(str(e) for e in errors)
    assert expected in rendered, f"{sidecar.name} wants {expected!r} in {rendered!r}"


def test_delivery_data_ok_states_carry_an_empty_error():
    """delivery-data.schema.json: error is "One line a person can act on; empty when ok", and
    story-deliver.md uses error only for the ok: false states (L2C-catchup-2). A valid fixture
    with ok: true and a message teaches a builder of vsdd story deliver to print an error on
    success."""
    for path in fixtures_of("delivery-data", "valid"):
        instance = json.loads(path.read_text(encoding="utf-8"))
        if instance.get("ok") is True:
            assert instance.get("error") == "", f"{path.name}: ok true with error {instance.get('error')!r}"


def test_refs_resolve_across_the_set():
    """The compose-heavy schemas reference their siblings by file name."""
    assert "finding.schema.json" in REGISTRY
    verdict_refs = json.dumps(REGISTRY["verdict.schema.json"])
    assert '"$ref": "finding.schema.json"' in verdict_refs
    step_refs = json.dumps(REGISTRY["step-result.schema.json"])
    assert '"$ref": "gate-result.schema.json"' in step_refs
    assert '"$ref": "spike-request.schema.json"' in step_refs


TRACKED_RECORD_SCHEMAS = ("closure-record", "cycle-record", "decision-record")


def _additional_false_paths(node, path: str = "$") -> list[str]:
    """The JSON paths of every ``additionalProperties: false`` in a raw
    schema, at any depth."""
    found: list[str] = []
    if isinstance(node, dict):
        for key, value in node.items():
            here = f"{path}.{key}"
            if key == "additionalProperties" and value is False:
                found.append(here)
            found.extend(_additional_false_paths(value, here))
    elif isinstance(node, list):
        for index, item in enumerate(node):
            found.extend(_additional_false_paths(item, f"{path}[{index}]"))
    return found


def test_tracked_record_schemas_stay_open_to_added_fields():
    """Decision D33 — several people on several machines share one
    repository — has a newer plugin writing tracked records that an older
    clone on another machine still reads: the tracked record schemas (the
    decision, closure and cycle records; the notes file ships no schema)
    stay open to added fields at every depth, and each description says a
    later version may add a field. A future ``additionalProperties: false``
    at any depth would make every older clone refuse the record as corrupt
    and its flush refuse to commit it."""
    for name in TRACKED_RECORD_SCHEMAS:
        raw = json.loads((SCHEMAS / f"{name}.schema.json").read_text(encoding="utf-8"))
        closed = _additional_false_paths(raw)
        assert not closed, f"{name}.schema.json closes added fields at {closed}"
        assert "may add a field" in raw["description"], f"{name}.schema.json description"


def test_tracked_record_store_round_trip_keeps_unknown_fields(tmp_path):
    """A tracked record carrying fields this schema version does not know yet
    (decision D33: the fixtures with an unknown field at the top level and
    one inside a nested object — for cycle-record at the top level only, its
    records have no nested objects) validates, and the store's read and
    write keep every unknown field: an older clone rewrites a newer record
    without trimming it."""
    from vsdd.state.store import read_record, write_record

    for name in TRACKED_RECORD_SCHEMAS:
        path = FIXTURES / name / "valid_future-fields.json"
        instance = json.loads(path.read_text(encoding="utf-8"))
        errors = validate(instance, schema_of(name), REGISTRY)
        assert not errors, f"{name}: " + "; ".join(str(e) for e in errors)

        written = tmp_path / name / "record.json"
        written.parent.mkdir(parents=True)
        write_record(written, instance, cap=None, schema=name)
        assert read_record(written, schema=name) == instance, name
