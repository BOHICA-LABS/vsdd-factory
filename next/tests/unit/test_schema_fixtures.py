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


def test_refs_resolve_across_the_set():
    """The compose-heavy schemas reference their siblings by file name."""
    assert "finding.schema.json" in REGISTRY
    verdict_refs = json.dumps(REGISTRY["verdict.schema.json"])
    assert '"$ref": "finding.schema.json"' in verdict_refs
    step_refs = json.dumps(REGISTRY["step-result.schema.json"])
    assert '"$ref": "gate-result.schema.json"' in step_refs
    assert '"$ref": "spike-request.schema.json"' in step_refs
