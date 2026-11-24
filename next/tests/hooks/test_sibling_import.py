"""Functional proof for the entries' sys.path insertion (R17; hooks.md 2.2, 2.4).

py/launch.sh exports PYTHONSAFEPATH=1, which stops uv from putting the script's
directory on sys.path. Each shipped entry inserts its own directory explicitly.
The plugin's py/ carries the real vsdd package (B3), so this test copies the
shipped entries into a temporary directory, gives the cli copy one extra line
that imports the sibling package, and runs the copy through the real launcher:
an import that fails under PYTHONSAFEPATH without the inserted line must
succeed with it. Nothing is generated under the plugin.
"""

from __future__ import annotations

import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from ._helpers import PLUGIN

ANCHOR = "sys.path.insert(0, str(Path(__file__).resolve().parent))"


def build_probe_case(temp: Path, with_insertion: bool) -> Path:
    """Copy the shipped py/ into temp/py and give the cli copy one extra line
    that imports the sibling vsdd package; with_insertion=False strips the
    sys.path line, recreating the defect the shipped code guards against.

    B3 and later ship the real py/vsdd/ package, and the probe imports it;
    the minimal stand-in only applies when the copy carries no package."""
    py_dst = temp / "py"
    shutil.copytree(PLUGIN / "py", py_dst, ignore=shutil.ignore_patterns("__pycache__"))

    vsdd_pkg = py_dst / "vsdd"
    if not vsdd_pkg.is_dir():
        vsdd_pkg.mkdir()
        (vsdd_pkg / "__init__.py").write_text(
            "SIBLING_MARK = 'imported-sibling'\n", encoding="utf-8"
        )

    cli = py_dst / "cli.py"
    text = cli.read_text(encoding="utf-8")
    assert ANCHOR in text, "the shipped cli.py must keep its sys.path insertion"
    text = text.replace(ANCHOR, ANCHOR + "\nimport vsdd", 1)
    if not with_insertion:
        # Recreate the defect: the import stays, the insertion goes.
        text = text.replace(ANCHOR, "pass  # insertion stripped by the test", 1)
    cli.write_text(text, encoding="utf-8")
    return py_dst


class TestSiblingImportSurvivesSafepath(unittest.TestCase):
    def run_launcher(self, py_dir: Path) -> subprocess.CompletedProcess:
        project = py_dir.parent / "proj"
        (project / ".factory").mkdir(parents=True, exist_ok=True)
        (project / ".factory" / "factory.toml").write_text("schema = 1\n", encoding="utf-8")
        env = {
            "PATH": "/usr/bin:/bin:/usr/local/bin:/opt/homebrew/bin",
            "HOME": str(py_dir.parent),
            "CLAUDE_PROJECT_DIR": str(project),
        }
        return subprocess.run(
            ["sh", str(py_dir / "launch.sh"), "cli"],
            capture_output=True, text=True, timeout=180, env=env, cwd=str(project),
        )

    def test_copied_entry_imports_sibling_package(self) -> None:
        if shutil.which("uv") is None:
            self.skipTest("uv is not installed")
        for with_insertion in (True, False):
            with self.subTest(with_insertion=with_insertion):
                with tempfile.TemporaryDirectory(prefix="vsdd-sibling-") as td:
                    py_dir = build_probe_case(Path(td), with_insertion)
                    result = self.run_launcher(py_dir)
                # The cli copy takes no arguments, so a successful import reaches
                # main(); the copied tree has no plugin manifest, so main exits 2
                # through the damaged-install message. Either way, the import must
                # not leave through uv as a traceback.
                if with_insertion:
                    self.assertNotIn("ModuleNotFoundError", result.stderr, result.stderr)
                    self.assertEqual(result.returncode, 2,
                                     f"import worked but usage flow broke: {result.stderr!r}")
                else:
                    self.assertIn("ModuleNotFoundError", result.stderr, result.stderr)
                    self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
