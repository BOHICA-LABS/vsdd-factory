"""A small JSON Schema checker (blueprint 4.1: ``schema.py``).

Ships in the plugin, standard library only. It validates the result schemas of
`<plugin>/schemas/*.json` — the single source the tool and the hook runner load
(blueprint 4.10) — and nothing else needs more than this subset:

- ``type``: ``object``, ``array``, ``string``, ``integer``, ``number``,
  ``boolean``, ``null`` (a single name or a list). ``integer`` also accepts a
  float with a zero fractional part; booleans are never integers or numbers.
- ``properties``, ``required``, ``additionalProperties`` (boolean or schema),
- ``enum`` (type-sensitive: ``True`` does not match ``1``),
- ``maxLength`` / ``minLength`` on strings, ``minimum`` / ``maximum`` on
  numbers, ``maxItems`` / ``minItems`` and ``items`` on arrays,
- ``anyOf`` / ``oneOf``: a list of subschemas, each checked as the schema itself,
- ``$ref``: a whole-document reference to a sibling schema by file name
  (``"finding.schema.json"``), resolved through a registry from
  :func:`load_registry`. Fragment pointers (``#/...``) are not supported.

The keyword set above is closed. The schemas are the single source for the
platform (the prelude literals the build generates) and for the tool and the
hook runner (loaded here), so a keyword the checker does not enforce must not
survive in a shipped schema: whoever adds one would see it hold in a workflow
step and be silently skipped by the tool and the hooks. :func:`load_registry`
therefore refuses every keyword outside the subset — and every boolean
subschema outside ``additionalProperties`` — naming the file and the JSON path.
Validation of a loaded schema is then honest by construction. Errors carry a
JSON path (``$.step.attempt``) and a readable message.
"""

from __future__ import annotations

import json
from pathlib import Path

__all__ = [
    "ValidationError",
    "validate",
    "is_valid",
    "load_registry",
    "SUPPORTED_KEYWORDS",
    "ANNOTATION_KEYWORDS",
    "check_subset",
]

_TYPES = ("object", "array", "string", "integer", "number", "boolean", "null")

#: Keywords the checker enforces. Anything else in a shipped schema is refused
#: at load time (blueprint 4.10: the files are the single source).
SUPPORTED_KEYWORDS = frozenset(
    {
        "type",
        "properties",
        "required",
        "additionalProperties",
        "enum",
        "maxLength",
        "minLength",
        "minimum",
        "maximum",
        "maxItems",
        "minItems",
        "items",
        "anyOf",
        "oneOf",
        "$ref",
    }
)

#: Annotations that decorate a schema without constraining anything.
ANNOTATION_KEYWORDS = frozenset({"$schema", "$id", "title", "description"})


class ValidationError:
    """One failed constraint: a JSON path and a readable message."""

    def __init__(self, path: str, message: str):
        self.path = path
        self.message = message

    def __str__(self) -> str:
        return f"{self.path}: {self.message}"


def _type_name(value: object) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "boolean"
    if isinstance(value, int):
        return "integer"
    if isinstance(value, float):
        return "number"
    if isinstance(value, str):
        return "string"
    if isinstance(value, list):
        return "array"
    return "object"


def _matches_type(value: object, wanted: str) -> bool:
    if wanted == "object":
        return isinstance(value, dict)
    if wanted == "array":
        return isinstance(value, list)
    if wanted == "string":
        return isinstance(value, str)
    if wanted == "boolean":
        return isinstance(value, bool)
    if wanted == "integer":
        return (isinstance(value, int) and not isinstance(value, bool)) or (
            isinstance(value, float) and value.is_integer()
        )
    if wanted == "number":
        return isinstance(value, (int, float)) and not isinstance(value, bool)
    if wanted == "null":
        return value is None
    return False


def _enum_equal(a: object, b: object) -> bool:
    if isinstance(a, bool) != isinstance(b, bool):
        return False
    return a == b


def _ref_value(registry: dict | None, ref: str) -> object:
    if registry is None or ref not in registry:
        return None
    return registry[ref]


def validate(
    instance: object,
    schema: dict,
    registry: dict | None = None,
    *,
    _path: str = "$",
    _depth: int = 0,
) -> list[ValidationError]:
    """Return every way ``instance`` fails ``schema``; an empty list is a pass."""
    if _depth > 64:
        return [ValidationError(_path, "schema nesting is deeper than 64 levels")]
    errors: list[ValidationError] = []

    def fail(message: str) -> None:
        errors.append(ValidationError(_path, message))

    declared = schema.get("type")
    if declared:
        names = declared if isinstance(declared, list) else [declared]
        for name in names:
            if name not in _TYPES:
                fail(f"schema declares unknown type {name!r}")
        if names and not any(_matches_type(instance, name) for name in names):
            got = _type_name(instance)
            fail(f"expected {' or '.join(names)}, got {got}")

    enum = schema.get("enum")
    if enum is not None and not any(_enum_equal(option, instance) for option in enum):
        fail(f"value {instance!r} is not one of {enum!r}")

    if isinstance(instance, str):
        max_length = schema.get("maxLength")
        if max_length is not None and len(instance) > max_length:
            fail(f"string is {len(instance)} characters; the maximum is {max_length}")
        min_length = schema.get("minLength")
        if min_length is not None and len(instance) < min_length:
            fail(f"string is {len(instance)} characters; the minimum is {min_length}")

    if isinstance(instance, (int, float)) and not isinstance(instance, bool):
        minimum = schema.get("minimum")
        if minimum is not None and instance < minimum:
            fail(f"{instance} is less than the minimum {minimum}")
        maximum = schema.get("maximum")
        if maximum is not None and instance > maximum:
            fail(f"{instance} is greater than the maximum {maximum}")

    if isinstance(instance, list):
        max_items = schema.get("maxItems")
        if max_items is not None and len(instance) > max_items:
            fail(f"array holds {len(instance)} items; the maximum is {max_items}")
        min_items = schema.get("minItems")
        if min_items is not None and len(instance) < min_items:
            fail(f"array holds {len(instance)} items; the minimum is {min_items}")
        items = schema.get("items")
        if isinstance(items, dict):
            for index, element in enumerate(instance):
                errors.extend(
                    validate(
                        element,
                        items,
                        registry,
                        _path=f"{_path}[{index}]",
                        _depth=_depth + 1,
                    )
                )

    if isinstance(instance, dict):
        for key in schema.get("required", []):
            if key not in instance:
                errors.append(
                    ValidationError(
                        f"{_path}.{key}", f"missing required property {key!r}"
                    )
                )
        properties = schema.get("properties") or {}
        for key, subschema in properties.items():
            if key in instance and isinstance(subschema, dict):
                errors.extend(
                    validate(
                        instance[key],
                        subschema,
                        registry,
                        _path=f"{_path}.{key}" if key else _path,
                        _depth=_depth + 1,
                    )
                )
        additional = schema.get("additionalProperties")
        if additional is False:
            known = set(properties)
            for key in instance:
                if key not in known:
                    errors.append(
                        ValidationError(
                            f"{_path}.{key}", f"unknown property {key!r}"
                        )
                    )
        elif isinstance(additional, dict):
            known = set(properties)
            for key, value in instance.items():
                if key not in known:
                    errors.extend(
                        validate(
                            value,
                            additional,
                            registry,
                            _path=f"{_path}.{key}",
                            _depth=_depth + 1,
                        )
                    )

    for combinator in ("anyOf", "oneOf"):
        branches = schema.get(combinator)
        if not isinstance(branches, list):
            continue
        results = [
            validate(instance, branch, registry, _path=_path, _depth=_depth + 1)
            for branch in branches
        ]
        passed = sum(1 for r in results if not r)
        if combinator == "anyOf" and passed == 0:
            fail(f"value matches none of the {len(branches)} anyOf branches")
        if combinator == "oneOf" and passed != 1:
            fail(
                f"value matches {passed} of the {len(branches)} oneOf branches; exactly one must match"
            )

    ref = schema.get("$ref")
    if isinstance(ref, str):
        target = _ref_value(registry, ref)
        if target is None:
            fail(
                f"unresolvable $ref {ref!r}"
                + ("" if registry is not None else " (no registry given)")
            )
        else:
            errors.extend(
                validate(instance, target, registry, _path=_path, _depth=_depth + 1)
            )

    return errors


def is_valid(instance: object, schema: dict, registry: dict | None = None) -> bool:
    """True when ``instance`` passes ``schema``."""
    return not validate(instance, schema, registry)


def _check_subset(schema: object, where: str, problems: list[str]) -> None:
    """Collect every unsupported keyword or boolean subschema under ``where``."""
    if isinstance(schema, dict):
        for key, value in schema.items():
            if key not in SUPPORTED_KEYWORDS and key not in ANNOTATION_KEYWORDS:
                problems.append(
                    f"{where}.{key}: unsupported keyword {key!r} "
                    f"(supported: {', '.join(sorted(SUPPORTED_KEYWORDS))}; "
                    f"annotations: {', '.join(sorted(ANNOTATION_KEYWORDS))})"
                )
                continue
            if key == "properties":
                if isinstance(value, dict):
                    for name, subschema in value.items():
                        if not isinstance(subschema, dict):
                            problems.append(
                                f"{where}.properties.{name}: a boolean or non-object "
                                "subschema is outside the subset"
                            )
                            continue
                        _check_subset(subschema, f"{where}.properties.{name}", problems)
                continue
            if key == "items":
                if not isinstance(value, dict) and value is not None:
                    problems.append(
                        f"{where}.items: a boolean subschema is outside the subset; "
                        "use a schema object"
                    )
                    continue
                if isinstance(value, dict):
                    _check_subset(value, f"{where}.items", problems)
                continue
            if key == "additionalProperties":
                if value is False or value is True:
                    continue  # the one boolean form the checker implements
                if isinstance(value, dict):
                    _check_subset(value, f"{where}.additionalProperties", problems)
                continue
            if key in ("anyOf", "oneOf"):
                if isinstance(value, list):
                    for index, branch in enumerate(value):
                        if not isinstance(branch, dict):
                            problems.append(
                                f"{where}.{key}[{index}]: a boolean subschema is "
                                "outside the subset"
                            )
                            continue
                        _check_subset(branch, f"{where}.{key}[{index}]", problems)
                continue


def check_subset(schema: dict) -> None:
    """Raise ``ValueError`` naming the JSON path of every keyword the checker
    does not enforce (blueprint 4.10: the files are the single source, so a
    constraint outside the subset must not survive into a shipped schema)."""
    problems: list[str] = []
    _check_subset(schema, "$", problems)
    if problems:
        raise ValueError("; ".join(problems))


def load_registry(directory: Path) -> dict[str, dict]:
    """Load every ``*.schema.json`` in ``directory``, keyed by file name.

    Refuses an empty or missing directory, a non-object schema file, invalid
    JSON (naming the file), and any keyword or boolean subschema outside the
    subset, with the offending JSON path — a shipped schema that carries one
    would hold in workflow steps and be silently skipped by the tool and the
    hooks.
    """
    directory = Path(directory)
    if not directory.is_dir():
        raise ValueError(f"schema directory {directory} does not exist")
    paths = sorted(directory.glob("*.schema.json"))
    if not paths:
        raise ValueError(f"schema directory {directory} holds no *.schema.json files")
    registry: dict[str, dict] = {}
    for path in paths:
        try:
            schema = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as exc:
            raise ValueError(f"{path.name}: invalid JSON: {exc}") from exc
        if not isinstance(schema, dict):
            raise ValueError(f"{path.name}: a schema must be a JSON object, got {type(schema).__name__}")
        try:
            check_subset(schema)
        except ValueError as exc:
            raise ValueError(f"{path.name}: {exc}") from exc
        registry[path.name] = schema
    return registry
