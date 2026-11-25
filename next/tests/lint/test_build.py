"""Regression tests for tools/build.py: marker integrity, insert/replace
parity, --check honesty, the stamped-marker guard, and schema literal stripping."""

from __future__ import annotations

import importlib.util
import shutil
import tempfile
import unittest
from pathlib import Path

NEXT = Path(__file__).resolve().parents[2]

_spec = importlib.util.spec_from_file_location("vsdd_build", NEXT / "tools" / "build.py")
build_mod = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(build_mod)

MARKERS = build_mod.PRELUDE_MARKERS
OPEN, CLOSE = MARKERS

META = "export const meta = {\n  name: 'x',\n  phases: [],\n}\n"
BODY = "const RUN = 1\nawait step()\n"
STAMPED = f"{META}{OPEN}\n{BODY}{CLOSE}\n"


def in_sandbox(original: Path, temp: Path) -> Path:
    """A copy of next/'s build inputs under temp, so the real tree is untouched.

    The virtual environment and the caches are not build inputs and are left out:
    copying them cost about eight seconds for every test in this file.
    """
    skip = shutil.ignore_patterns(".venv", "__pycache__", ".pytest_cache", ".ruff_cache", "node_modules")
    shutil.copytree(original, temp / "copy", ignore=skip)
    return temp / "copy"


class TestMarkerIntegrity(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="vsdd-buildtest-")
        self.addCleanup(self._tmp.cleanup)
        self.root = in_sandbox(NEXT, Path(self._tmp.name))
        self._build = self._reload()

    def _reload(self):
        spec = importlib.util.spec_from_file_location("vsdd_build_local", self.root / "tools" / "build.py")
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        return mod

    def test_stamped_workflow_in_plugin_workflows_is_allowed(self):
        """A correctly stamped workflow in <plugin>/workflows/ must build clean:
        the guard allows by exact file, not by parent directory."""
        target = self.root / "plugins" / "vsdd-factory" / "workflows" / "toy-story.js"
        target.write_text(STAMPED, encoding="utf-8")
        drifted, problems = self._build.build(check_only=True)
        self.assertEqual(problems, [])

    def test_marker_in_tests_is_detected(self):
        probe = self.root / "tests" / "workflows" / "stray.mjs"
        probe.write_text(f"// {OPEN}\nexport const meta = {{}}\n", encoding="utf-8")
        drifted, problems = self._build.build(check_only=True)
        self.assertTrue(any("stray.mjs" in p and "outside the stamping scope" in p for p in problems), problems)

    def test_both_markers_scanned_outside_the_scope(self):
        probe = self.root / "tests" / "workflows" / "stray-close.mjs"
        probe.write_text(f"// {CLOSE}\n", encoding="utf-8")
        drifted, problems = self._build.build(check_only=True)
        self.assertTrue(any("stray-close.mjs" in p for p in problems), problems)

    def test_check_mode_exits_nonzero_on_problems(self):
        probe = self.root / "tests" / "workflows" / "stray2.mjs"
        probe.write_text(f"// {OPEN}\n", encoding="utf-8")
        code = self._build.main(["--check"])
        self.assertEqual(code, 1)

    def test_replace_between_missing_closing_marker_raises_and_never_writes(self):
        broken = f"{META}{OPEN}\n{BODY}"  # no closing marker
        with self.assertRaises(build_mod.MarkerProblem):
            build_mod._replace_between(broken, MARKERS, "new")
        path = self.root / "tests" / "workflows" / "toys" / "toy-build.js"
        original = path.read_text()
        path.write_text(original.replace(CLOSE, ""), encoding="utf-8")
        drifted, problems = self._build.build(check_only=False)
        self.assertTrue(any("toy-build.js" in p for p in problems), problems)
        # the file was never rewritten to a truncated tail
        self.assertIn("toy", path.read_text())

    def test_duplicate_opening_marker_is_a_problem(self):
        doubled = f"{META}{OPEN}\n{OPEN}\n{BODY}{CLOSE}\n"
        with self.assertRaises(build_mod.MarkerProblem):
            build_mod._replace_between(doubled, MARKERS, "new")

    def test_insert_then_check_is_clean_and_idempotent(self):
        path = self.root / "tests" / "workflows" / "toys" / "toy-build.js"
        text = META + BODY  # unstamped: build inserts the region
        path.write_text(text, encoding="utf-8")
        drifted, problems = self._build.build(check_only=False)
        self.assertEqual(problems, [], problems)
        self.assertIn(path.name, " ".join(drifted))
        stamped_once = path.read_text()
        # The inserted file loads as JavaScript syntax: balanced braces from the
        # stamped region ride on the same generator, and the marker line carries
        # only comments.
        for line in stamped_once.splitlines():
            if MARKERS[0] in line or MARKERS[1] in line:
                stripped = line.replace(MARKERS[0], "").replace(MARKERS[1], "").strip()
                if stripped and not stripped.startswith("//"):
                    self.fail(f"non-comment text on a marker line: {line!r}")
        # a second build writes nothing and --check agrees
        drifted, problems = self._build.build(check_only=True)
        self.assertEqual(problems, [])
        self.assertEqual(drifted, [])
        self.assertEqual(path.read_text(), stamped_once)

    def test_property_named_description_survives_the_literal(self):
        schema = {
            "type": "object",
            "properties": {"description": {"type": "string"}, "size": {"type": "integer"}},
            "required": ["description", "size"],
            "description": "annotated",
        }
        stripped = build_mod._strip_schema_node(schema, {})
        self.assertIn("description", stripped["properties"])
        self.assertNotIn("description", stripped)


class TestHandLiteralGuard(unittest.TestCase):
    """CATCHUP-R2: the build alone writes S_* constants (result-schemas.md: no S_*
    literal is written by hand), and every shipped schema file is accounted for in
    the literal mapping, so a schema without a mapping cannot silently lose its literal."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="vsdd-buildtest-")
        self.addCleanup(self._tmp.cleanup)
        self.root = in_sandbox(NEXT, Path(self._tmp.name))
        self._build = self._reload()

    def _reload(self):
        spec = importlib.util.spec_from_file_location("vsdd_build_guard", self.root / "tools" / "build.py")
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
        return mod

    def test_shipped_tree_is_clean(self):
        drifted, problems = self._build.build(check_only=True)
        self.assertEqual(problems, [], problems)

    def test_hand_written_literal_in_the_prelude_is_reported(self):
        prelude = self.root / "workflow-lib" / "prelude.js"
        text = prelude.read_text()
        # Outside both generated blocks: appending after the last one is the hand-written spot.
        marker = "S_HANDMADE_PLACEHOLDER"
        prelude.write_text(text + f"\nconst {marker} = {{ 'type': 'object' }}\n", encoding="utf-8")
        drifted, problems = self._build.build(check_only=True)
        self.assertTrue(any(marker in p for p in problems), problems)
        # --check exits nonzero on it.
        self.assertEqual(self._build.main(["--check"]), 1)

    def test_hand_written_literal_in_a_workflow_body_is_reported(self):
        toy = self.root / "tests" / "workflows" / "toys" / "toy-build.js"
        text = toy.read_text()
        toy.write_text(text.replace("// <<< stamped:prelude", "// <<< stamped:prelude\nconst S_BODYMADE = {}", 1), encoding="utf-8")
        drifted, problems = self._build.build(check_only=True)
        self.assertTrue(any("S_BODYMADE" in p for p in problems), problems)

    def test_generated_literals_themselves_are_not_reported(self):
        drifted, problems = self._build.build(check_only=True)
        self.assertFalse(any("S_FINDING" in p or "S_GATE" in p for p in problems), problems)

    def test_shipped_schema_without_a_mapping_is_an_error(self):
        orphan = self.root / "plugins" / "vsdd-factory" / "schemas" / "orphan.schema.json"
        orphan.write_text('{"type": "object"}', encoding="utf-8")
        drifted, problems = self._build.build(check_only=True)
        self.assertTrue(any("orphan.schema.json" in p and "no literal mapping covers" in p for p in problems), problems)

    def test_mapping_without_a_shipped_file_is_an_error(self):
        self._build.LITERAL_NAMES["ghost.schema.json"] = "S_GHOST"
        drifted, problems = self._build.build(check_only=True)
        self.assertTrue(any("ghost.schema.json" in p and "not shipped" in p for p in problems), problems)


if __name__ == "__main__":
    unittest.main()
