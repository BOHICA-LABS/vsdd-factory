"""Edges of the small JSON Schema checker (``vsdd.schema``)."""

import json

import pytest

from vsdd.schema import ValidationError, check_subset, is_valid, load_registry, validate

OBJECT = {
    "type": "object",
    "properties": {
        "name": {"type": "string", "minLength": 1, "maxLength": 10},
        "count": {"type": "integer", "minimum": 1},
        "score": {"type": "number", "maximum": 1},
        "flag": {"type": "boolean"},
        "tags": {"type": "array", "items": {"type": "string"}, "maxItems": 2},
    },
    "required": ["name"],
}


def paths(errors):
    return [error.path for error in errors]


class TestTypes:
    def test_bool_is_not_an_integer(self):
        errors = validate(True, {"type": "integer"})
        assert any("boolean" in str(e) for e in errors)

    def test_bool_is_not_a_number(self):
        assert not is_valid(True, {"type": "number"})

    def test_integer_accepts_integral_float(self):
        assert is_valid(2.0, {"type": "integer"})
        assert not is_valid(2.5, {"type": "integer"})

    def test_type_list(self):
        assert is_valid(None, {"type": ["string", "null"]})
        assert not is_valid(3, {"type": ["string", "null"]})

    def test_object_array_string_null(self):
        assert is_valid({}, {"type": "object"})
        assert is_valid([], {"type": "array"})
        assert is_valid("x", {"type": "string"})
        assert is_valid(None, {"type": "null"})


class TestConstraints:
    def test_required(self):
        errors = validate({}, OBJECT)
        assert paths(errors) == ["$.name"]

    def test_max_length(self):
        errors = validate({"name": "x" * 11}, OBJECT)
        assert paths(errors) == ["$.name"]

    def test_min_length(self):
        errors = validate({"name": ""}, OBJECT)
        assert paths(errors) == ["$.name"]

    def test_minimum_and_maximum(self):
        assert paths(validate({"name": "x", "count": 0}, OBJECT)) == ["$.count"]
        assert paths(validate({"name": "x", "score": 1.5}, OBJECT)) == ["$.score"]

    def test_array_items_and_max_items(self):
        base = {"name": "x"}
        assert paths(validate({**base, "tags": [1]}, OBJECT)) == ["$.tags[0]"]
        assert paths(validate({**base, "tags": ["a", "b", "c"]}, OBJECT)) == ["$.tags"]

    def test_enum_is_type_sensitive(self):
        schema = {"enum": ["on", "off"]}
        assert is_valid("on", schema)
        assert not is_valid("On", schema)

    def test_boolean_does_not_match_integer_enum_member(self):
        assert not is_valid(True, {"enum": [1]})
        assert is_valid(1, {"enum": [1]})

    def test_additional_properties_false(self):
        schema = {
            "type": "object",
            "properties": {"name": {}, "a": {}},
            "additionalProperties": False,
        }
        errors = validate({"name": "x", "a": 1, "b": 2}, schema)
        assert paths(errors) == ["$.b"]

    def test_deep_nesting_paths(self):
        schema = {"type": "object", "properties": {"rows": {"type": "array", "items": {"type": "object", "properties": {"n": {"type": "integer"}}}}}}
        errors = validate({"rows": [{"n": "x"}]}, schema)
        assert paths(errors) == ["$.rows[0].n"]


class TestRefs:
    def test_ref_via_registry(self):
        registry = {"inner.schema.json": {"type": "integer"}}
        schema = {"type": "array", "items": {"$ref": "inner.schema.json"}}
        assert is_valid([1, 2], schema, registry)
        errors = validate([1, "x"], schema, registry)
        assert paths(errors) == ["$[1]"]

    def test_unresolvable_ref(self):
        errors = validate({}, {"$ref": "missing.schema.json"}, {})
        assert "unresolvable" in str(errors[0])

    def test_ref_without_registry(self):
        errors = validate({}, {"$ref": "missing.schema.json"})
        assert "unresolvable" in str(errors[0])


class TestBehaviour:
    def test_unknown_keywords_are_refused_by_the_subset_check(self):
        """The checker ignores nothing silently: a keyword outside the subset is
        refused at load time with its JSON path (the files are the single source
        for the platform and for the tool and hook runner alike)."""
        with pytest.raises(ValueError) as excinfo:
            check_subset({"type": "string", "pattern": "^x$", "format": "uri"})
        assert "$.pattern" in str(excinfo.value)
        assert "$.format" in str(excinfo.value)

    def test_typo_of_a_supported_keyword_is_refused(self):
        with pytest.raises(ValueError) as excinfo:
            check_subset({"type": "object", "properties": {"x": {"type": "string"}}, "requried": ["x"]})
        assert "$.requried" in str(excinfo.value)

    def test_annotations_are_accepted(self):
        check_subset(
            {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "$id": "x.schema.json",
                "title": "T",
                "description": "D",
                "type": "string",
            }
        )

    def test_boolean_subschemas_outside_additional_properties_are_refused(self):
        with pytest.raises(ValueError, match=r"\.properties\.a"):
            check_subset({"type": "object", "properties": {"a": False}})
        with pytest.raises(ValueError, match=r"\.items"):
            check_subset({"type": "array", "items": False})

    def test_additional_properties_false_is_accepted(self):
        check_subset({"type": "object", "properties": {}, "additionalProperties": False})

    def test_combinators_are_part_of_the_subset(self):
        check_subset({"anyOf": [{"type": "string"}, {"type": "integer"}]})
        with pytest.raises(ValueError, match=r"\.anyOf\[1\]"):
            check_subset({"anyOf": [{"type": "string"}, True]})


class TestCombinators:
    def test_any_of_passes_on_one_matching_branch(self):
        schema = {"anyOf": [{"type": "string"}, {"type": "integer"}]}
        assert is_valid("x", schema)
        assert is_valid(3, schema)
        assert not is_valid(True, schema)
        assert not is_valid([], schema)

    def test_one_of_exactly_one_branch(self):
        schema = {"oneOf": [{"type": "integer", "minimum": 2}, {"type": "integer"}]}
        assert is_valid(1, schema)
        errors = validate(3, schema)  # matches both: refused
        assert len(errors) == 1
        assert "exactly one" in str(errors[0])


class TestRegistryLoading:
    def test_missing_directory_raises(self, tmp_path):
        with pytest.raises(ValueError, match="does not exist"):
            load_registry(tmp_path / "absent")

    def test_empty_directory_raises(self, tmp_path):
        with pytest.raises(ValueError, match="holds no"):
            load_registry(tmp_path)

    def test_broken_json_names_the_file(self, tmp_path):
        (tmp_path / "broken.schema.json").write_text("{oops", encoding="utf-8")
        with pytest.raises(ValueError) as excinfo:
            load_registry(tmp_path)
        assert "broken.schema.json" in str(excinfo.value)

    def test_non_object_schema_raises(self, tmp_path):
        (tmp_path / "bool.schema.json").write_text("true", encoding="utf-8")
        with pytest.raises(ValueError, match="JSON object"):
            load_registry(tmp_path)

    def test_unknown_keyword_names_the_file_and_path(self, tmp_path):
        (tmp_path / "fancy.schema.json").write_text(
            json.dumps({"type": "string", "pattern": "^x$"}), encoding="utf-8"
        )
        with pytest.raises(ValueError) as excinfo:
            load_registry(tmp_path)
        assert "fancy.schema.json" in str(excinfo.value)
        assert "$.pattern" in str(excinfo.value)

    def test_clean_directory_loads(self, tmp_path):
        (tmp_path / "simple.schema.json").write_text(
            json.dumps({"type": "string", "minLength": 1}), encoding="utf-8"
        )
        registry = load_registry(tmp_path)
        assert is_valid("x", registry["simple.schema.json"])

    def test_validation_error_rendering(self):
        error = ValidationError("$.step.attempt", "boom")
        assert str(error) == "$.step.attempt: boom"

    def test_non_object_instance_with_properties_is_an_error_on_type_only(self):
        errors = validate("not-an-object", OBJECT)
        assert len(errors) == 1
        assert "expected object" in str(errors[0])
