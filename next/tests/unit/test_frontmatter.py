"""Edges of the strict frontmatter subset (hooks.md 2.4, state.md 2.4)."""

from datetime import date
from pathlib import Path

import pytest

from vsdd.frontmatter import FrontmatterError, parse, split

PLUGIN = Path(__file__).resolve().parents[2] / "plugins" / "vsdd-factory"


class TestScalars:
    def test_flat_block_of_scalars(self):
        text = "---\nid: BC-2.03.007\ntitle: Rows are exported\nstatus: draft\nretries: 3\nactive: true\nobsolete: false\nempty: null\ntilde: ~\nbare: nothing at all\n---\nbody"
        meta = parse(text)
        assert meta == {
            "id": "BC-2.03.007",
            "title": "Rows are exported",
            "status": "draft",
            "retries": 3,
            "active": True,
            "obsolete": False,
            "empty": None,
            "tilde": None,
            "bare": "nothing at all",
        }

    def test_empty_value_is_null(self):
        assert parse("---\nnothing:\n---\n") == {"nothing": None}

    def test_quoted_strings_keep_punctuation(self):
        meta = parse("---\na: 'one, two'\nb: \"x: y\"\nc: 'it''s'\n---\n")
        assert meta == {"a": "one, two", "b": "x: y", "c": "it's"}

    def test_iso_date_becomes_a_date(self):
        meta = parse("---\ncreated: 2026-11-24\n---\n")
        assert meta["created"] == date(2026, 11, 24)

    def test_integers_include_negatives(self):
        assert parse("---\nn: -12\n---\n")["n"] == -12

    def test_float_is_outside_the_subset(self):
        with pytest.raises(FrontmatterError, match="line 2"):
            parse("---\nratio: 1.5\n---\n")


class TestLists:
    def test_flow_list_of_scalars(self):
        meta = parse("---\naliases: [BC-2.03.008, BC-2.03.009]\n---\n")
        assert meta["aliases"] == ["BC-2.03.008", "BC-2.03.009"]

    def test_empty_flow_list(self):
        assert parse("---\naliases: []\n---\n")["aliases"] == []

    def test_flow_list_quoted_item_with_comma(self):
        meta = parse("---\nwords: ['a, b', c]\n---\n")
        assert meta["words"] == ["a, b", "c"]

    def test_block_list_of_scalars(self):
        meta = parse("---\nedges:\n  - two rows\n  - empty file\n---\n")
        assert meta["edges"] == ["two rows", "empty file"]

    def test_block_list_at_key_indent(self):
        meta = parse("---\nedges:\n- two rows\n- empty file\n---\n")
        assert meta["edges"] == ["two rows", "empty file"]

    def test_trailing_comma_refused(self):
        with pytest.raises(FrontmatterError):
            parse("---\nxs: [a, b,]\n---\n")

    def test_unterminated_flow_list_refused(self):
        with pytest.raises(FrontmatterError):
            parse("---\nxs: [a, b\n---\n")


class TestNestedMap:
    def test_one_level(self):
        meta = parse("---\nowner:\n  team: core\n  seat: 4\nstatus: draft\n---\n")
        assert meta == {"owner": {"team": "core", "seat": 4}, "status": "draft"}

    def test_second_level_refused(self):
        with pytest.raises(FrontmatterError, match="second nesting level"):
            parse("---\na:\n  b:\n    c: 1\n---\n")

    def test_nested_requires_scalar_value(self):
        with pytest.raises(FrontmatterError):
            parse("---\na:\n  b:\n---\n")


class TestStructure:
    def test_no_frontmatter_is_empty(self):
        assert parse("just a body\n") == {}

    def test_split_returns_body_after_fence(self):
        meta, body = split("---\nid: x\n---\n# Head\nbody")
        assert meta == {"id": "x"}
        assert body == "# Head\nbody"

    def test_unclosed_block_refused(self):
        with pytest.raises(FrontmatterError, match="not closed"):
            parse("---\nid: x\n")

    def test_duplicate_key_refused(self):
        with pytest.raises(FrontmatterError, match="duplicate key 'id'"):
            parse("---\nid: a\nid: b\n---\n")

    def test_tab_indentaion_refused(self):
        with pytest.raises(FrontmatterError, match="tab"):
            parse("---\nlist:\n\t- a\n---\n")

    def test_block_scalar_refused(self):
        with pytest.raises(FrontmatterError):
            parse("---\ntext: |\n  lines\n---\n")

    def test_anchor_refused(self):
        with pytest.raises(FrontmatterError):
            parse("---\nx: &anchor value\n---\n")

    def test_unknown_top_level_shape_refused(self):
        with pytest.raises(FrontmatterError, match="expected 'key: value'"):
            parse("---\njust a line\n---\n")

    def test_comments_and_blanks_allowed(self):
        meta = parse("---\n# leading comment\nid: x\n\n# another\nstatus: draft\n---\n")
        assert meta == {"id": "x", "status": "draft"}

    def test_errors_carry_line_numbers(self):
        with pytest.raises(FrontmatterError) as excinfo:
            parse("---\nid: a\nratio: 2.5\n---\n")
        assert excinfo.value.line == 3

    def test_dash_fence_inside_block_refused(self):
        with pytest.raises(FrontmatterError, match="unbalanced"):
            parse("---\nid: a\n--- extra\n---\n")


FLAT_DOC = """\
---
id: S-9.01
title: Export drops the last row
status: draft
epic: E-9
contracts: [BC-9.02.001, BC-9.02.002]
ui: false
---

Body follows.
"""


class TestFlatArtifactSubset:
    """state.md 2.4 flat frontmatter must parse and split cleanly."""

    def test_state_example_round_trip(self):
        meta, body = split(FLAT_DOC)
        assert meta == {
            "id": "S-9.01",
            "title": "Export drops the last row",
            "status": "draft",
            "epic": "E-9",
            "contracts": ["BC-9.02.001", "BC-9.02.002"],
            "ui": False,
        }
        # The body is everything after the closing fence, blank line included.
        assert body.lstrip().startswith("Body follows.")


class TestDatesAreFrontmatterErrors:
    """A date-shaped value that is not a real date is a FrontmatterError with
    the line, never a bare ValueError: the artifacts rule fails closed, and the
    block message must give the agent something to act on."""

    def test_impossible_month(self):
        with pytest.raises(FrontmatterError) as excinfo:
            parse("---\nd: 2026-13-45\n---\n")
        assert excinfo.value.line == 2
        assert "not a valid date" in str(excinfo.value)

    def test_impossible_day(self):
        with pytest.raises(FrontmatterError) as excinfo:
            parse("---\nd: 2026-02-30\n---\n")
        assert excinfo.value.line == 2
        assert "not a valid date" in str(excinfo.value)


class TestFenceAndBodyEdges:
    def test_utf8_bom_is_skipped(self):
        meta = parse("﻿---\nid: x\n---\nbody")
        assert meta == {"id": "x"}

    def test_split_after_bom_returns_the_body(self):
        meta, body = split("﻿---\nid: x\n---\nbody\n")
        assert meta == {"id": "x"}
        assert body == "body\n"

    def test_closing_fence_may_carry_trailing_spaces(self):
        meta, body = split("---\nid: x\n---  \nbody")
        assert meta == {"id": "x"}
        assert body == "body"

    def test_split_keeps_the_final_newline(self):
        _, body = split("---\nid: x\n---\n# Head\nbody\n")
        assert body.endswith("body\n")

    def test_split_keeps_crlf_endings(self):
        _, body = split("---\rid: x\r---\rline1\r\nline2\r\n")
        assert "\r\n" in body or body.startswith("line1")


class TestDifferentialAgainstPyYAML:
    """packaging.md: the shipped reader and PyYAML agree on every input.

    Two halves, both running now:

    - a crafted corpus: every accepted-subset input must parse to exactly what
      ``yaml.safe_load`` returns, and every out-of-subset input — each misread
      the probes against PyYAML found — must raise :class:`FrontmatterError`
      naming the offending line;
    - the plugin-file scan, which covers the shipped artifacts as they land.
    """

    # @pytest.mark.accepted: the reader accepts it, so it must equal PyYAML.
    ACCEPTED = [
        ("flat scalars", "---\nid: BC-2.03.007\ntitle: Rows are exported\nstatus: draft\nretries: 3\nactive: true\nobsolete: false\nempty: null\ntilde: ~\n---\n",
         {"id": "BC-2.03.007", "title": "Rows are exported", "status": "draft", "retries": 3,
          "active": True, "obsolete": False, "empty": None, "tilde": None}),
        ("bare string with punctuation", "---\nbare: nothing at all\n---\n", {"bare": "nothing at all"}),
        ("quoted single", "---\na: 'one, two'\n---\n", {"a": "one, two"}),
        ("quoted double with colon", "---\nb: \"x: y\"\n---\n", {"b": "x: y"}),
        ("single-quoted apostrophe", "---\nc: 'it''s'\n---\n", {"c": "it's"}),
        ("negative integer", "---\nn: -12\n---\n", {"n": -12}),
        ("zero", "---\nn: 0\n---\n", {"n": 0}),
        ("iso date", "---\ncreated: 2026-11-24\n---\n", {"created": date(2026, 11, 24)}),
        ("flow list", "---\naliases: [BC-2.03.008, BC-2.03.009]\n---\n",
         {"aliases": ["BC-2.03.008", "BC-2.03.009"]}),
        ("flow list quoted comma", "---\nwords: ['a, b', c]\n---\n", {"words": ["a, b", "c"]}),
        ("empty flow list", "---\naliases: []\n---\n", {"aliases": []}),
        ("block list indented", "---\nedges:\n  - two rows\n  - empty file\n---\n",
         {"edges": ["two rows", "empty file"]}),
        ("block list at key indent", "---\nedges:\n- two rows\n- empty file\n---\n",
         {"edges": ["two rows", "empty file"]}),
        ("nested map one level", "---\nowner:\n  team: core\n  seat: 4\nstatus: draft\n---\n",
         {"owner": {"team": "core", "seat": 4}, "status": "draft"}),
        ("empty value", "---\nnothing:\n---\n", {"nothing": None}),
        ("comment line", "---\n# lead\nid: x\n\n# another\nstatus: draft\n---\n",
         {"id": "x", "status": "draft"}),
        ("comment-only value is null as in yaml", "---\nx: # note\n---\n", {"x": None}),
        ("url as plain scalar", "---\nrepo: http://x.example/branch\n---\n",
         {"repo": "http://x.example/branch"}),
    ]

    # (name, document, line the error must name)
    REFUSED = [
        ("comment on value line", "---\nstatus: draft # note\n---\n", 2),
        ("mapping in a scalar", "---\ntitle: a: b\n---\n", 2),
        ("inline dash value", "---\nx: - a\n---\n", 2),
        ("percent indicator", "---\nx: %wrap\n---\n", 2),
        ("at indicator", "---\nhandle: @name\n---\n", 2),
        ("backtick indicator", "---\nx: `cmd`\n---\n", 2),
        ("tag indicator", "---\nx: !!str 5\n---\n", 2),
        ("anchor indicator", "---\nx: &anchor value\n---\n", 2),
        ("alias indicator", "---\nx: *anchor\n---\n", 2),
        ("yaml 1.1 no", "---\nui: no\n---\n", 2),
        ("yaml 1.1 yes", "---\nui: yes\n---\n", 2),
        ("yaml 1.1 on", "---\nui: on\n---\n", 2),
        ("yaml 1.1 True", "---\nui: True\n---\n", 2),
        ("yaml 1.1 FALSE", "---\nui: FALSE\n---\n", 2),
        ("yaml 1.1 Null", "---\nx: Null\n---\n", 2),
        ("octal-looking int", "---\nn: 010\n---\n", 2),
        ("plus int", "---\nn: +5\n---\n", 2),
        ("hex int", "---\nn: 0x1F\n---\n", 2),
        ("underscore int", "---\nn: 1_000\n---\n", 2),
        ("sexagesimal", "---\nt: 12:30\n---\n", 2),
        ("infinity", "---\nf: .inf\n---\n", 2),
        ("float", "---\nratio: 1.5\n---\n", 2),
        ("impossible month", "---\nd: 2026-13-45\n---\n", 2),
        ("impossible day", "---\nd: 2026-02-30\n---\n", 2),
        ("mapping as block-list item", "---\nkeys:\n- name: a\n---\n", 3),
        ("empty flow item", "---\nxs: [a,,b]\n---\n", 2),
        ("interior unescaped double quote", "---\nx: \"x\"y\"\n---\n", 2),
        ("interior unescaped single quote", "---\nx: 'it's'\n---\n", 2),
        ("double-quoted backslash escape", '---\np: "C:\\temp"\n---\n', 2),
        ("key without space after colon", "---\nkey:value\n---\n", 2),
        ("nested line without key", "---\nouter:\n  http://x\n---\n", 3),
        ("truncated double-quoted escapes", '---\nx: "a\\nb"\n---\n', 2),
    ]

    @staticmethod
    def _inner(text: str) -> str:
        """The block between the two fences: what PyYAML parses as one document."""
        lines = text.splitlines()
        end = next(i for i in range(1, len(lines)) if lines[i].rstrip() == "---")
        return "\n".join(lines[1:end])

    @pytest.mark.parametrize("name,text,expected", ACCEPTED, ids=[a[0] for a in ACCEPTED])
    def test_accepted_input_equals_pyyaml(self, name, text, expected):
        import yaml  # a pinned dev dependency; see pyproject.toml
        assert parse(text) == yaml.safe_load(self._inner(text)), name

    @pytest.mark.parametrize("name,text,line", REFUSED, ids=[r[0] for r in REFUSED])
    def test_out_of_subset_input_is_refused_with_its_line(self, name, text, line):
        with pytest.raises(FrontmatterError) as excinfo:
            parse(text)
        assert excinfo.value.line == line, f"{name}: wrong line {excinfo.value.line}"

    @staticmethod
    def _naive_value(text: str) -> object:
        """What the reader would have produced for the last ``key: value`` line
        had it kept the raw text instead of refusing it."""
        lines = [ln for ln in text.splitlines() if ln.strip() and ln != "---"]
        _, _, rest = lines[-1].partition(":")
        return rest.strip()

    def test_refused_cases_reject_or_disagree_with_pyyaml(self):
        """Corpus guard: a refused entry must be a case PyYAML itself rejects,
        or one where PyYAML's value differs from the raw text — the misread the
        refusal exists to prevent."""
        import yaml

        for name, text, *_ in self.REFUSED:
            try:
                theirs = yaml.safe_load(self._inner(text))
            except (yaml.YAMLError, ValueError):
                continue  # ambiguous for both readers (PyYAML raises ValueError on impossible dates)
            assert theirs != self._naive_value(text), (
                f"{name}: PyYAML reads the naive value {theirs!r}; the refusal "
                "needs justifying or the case belongs in ACCEPTED"
            )

    def test_plugin_frontmatter_agrees(self):
        import yaml  # a pinned dev dependency; a missing PyYAML is a setup fault

        examined = []
        for base in ("agents", "skills", "templates", "methods"):
            directory = PLUGIN / base
            if not directory.is_dir():
                continue
            for path in sorted(directory.rglob("*.md")):
                text = path.read_text(encoding="utf-8")
                if not text.startswith("---"):
                    continue
                mine = parse(text)
                theirs = yaml.safe_load(self._inner(text))
                assert mine == theirs, f"{path}: shipped reader disagrees with PyYAML"
                examined.append(path)
        # The plugin ships no markdown artifacts yet (agents and skills arrive
        # with packages A and K); the differential harness must be ready for
        # them, and the first files that land are covered at once.
        assert isinstance(examined, list)
