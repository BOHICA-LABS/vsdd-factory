"""Run every tools/check.py check as a test.

One source of truth: this module dynamically builds a unittest.TestCase from
the check registry in tools/check.py, so pytest (Layer 3 lint, packaging.md
2.6) and `uv run python tools/check.py` execute exactly the same assertions
and can never drift apart.
"""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path

NEXT = Path(__file__).resolve().parents[2]

_spec = importlib.util.spec_from_file_location("vsdd_check", NEXT / "tools" / "check.py")
_check = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(_check)


class TestRepositoryChecks(unittest.TestCase):
    def test_every_registered_check(self) -> None:
        expect_registered = getattr(_check, "REGISTRY", None)
        assert expect_registered is not None, "tools/check.py must expose REGISTRY"
        for fn in expect_registered:
            with self.subTest(check=fn.__name__):
                fn()

    def test_registry_covers_the_expected_areas(self) -> None:
        names = {fn.__name__ for fn in _check.REGISTRY}
        for expected in (
            "skeleton_layout",
            "plugin_manifest",
            "plugin_layout",
            "licence_parity",
            "mcp_configuration",
            "shell_syntax",
            "shell_lint",
            "nothing_generated_under_plugin",
            "shipped_files_are_not_gitignored",
            "entries_self_register_on_sys_path",
            "shipped_python_is_offline_stdlib",
            "launcher_inert_outside_factory_projects",
            "launcher_blocks_without_uv",
            "launcher_modes_on_a_live_project",
        ):
            self.assertIn(expected, names)


if __name__ == "__main__":
    unittest.main()
