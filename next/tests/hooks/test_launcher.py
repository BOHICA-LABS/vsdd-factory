"""Launcher behaviour tests (B2; hooks.md 2.2, 2.9, 2.12, 2.14).

Every behavioural case runs the real py/launch.sh under sh, bash and dash with a fake
uv first on PATH. The fake records its argv and exits with a chosen code, so the
exit-code matrix (0, 1, 2, 10, 13, 137) and the messages are guarded by tests, not by
hand-verifying on one machine.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from ._helpers import INTERPRETERS, launch, make_fake_uv, make_project

GATE_MODES = ("gate", "brief", "notify")


class FakeUvCase(unittest.TestCase):
    """Common machinery: a project tree, a fake uv, and a log of its invocations."""

    def setUp(self) -> None:
        tmp = tempfile.TemporaryDirectory(prefix="vsdd-launcher-")
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.project = make_project(self.root)
        self.bin_dir = self.root / "bin"
        self.fake_log = self.root / "uv-calls.log"
        self.uv = make_fake_uv(self.bin_dir, self.fake_log)
        self.events = self.root / "events"

    def run_hook(self, mode: str, interpreter: str, *, fake_rc=None, fake_find_rc=None, **kw):
        return launch(
            interpreter,
            ["hook", mode],
            self.project,
            path=f"{self.bin_dir}:/usr/bin:/bin",
            events_dir=self.events,
            fake_log=self.fake_log,
            fake_rc=fake_rc,
            fake_find_rc=fake_find_rc,
            **kw,
        )

    def run_cli(self, interpreter: str, *args: str, **kw):
        return launch(
            interpreter,
            ["cli", *args],
            self.project,
            path=f"{self.bin_dir}:/usr/bin:/bin",
            events_dir=self.events,
            fake_log=self.fake_log,
            **kw,
        )

    def event_lines(self) -> list[dict]:
        files = sorted(self.events.glob("*.jsonl"))
        self.assertTrue(files, "no event log was written")
        return [json.loads(line) for line in files[0].read_text().splitlines() if line.strip()]

    def uv_calls(self) -> list[str]:
        if not self.fake_log.exists():
            return []
        return self.fake_log.read_text().splitlines()


class TestUnknownMode(FakeUvCase):
    """R11: an unknown hook mode must fail closed, never silently exit 0."""

    def test_unknown_mode_fails_closed_in_every_interpreter(self) -> None:
        for interpreter in INTERPRETERS:
            for bad in ("Gate", "gate ", "gates"):
                with self.subTest(shell=interpreter, mode=bad):
                    result = launch(
                        interpreter, ["hook", bad], self.project,
                        path=f"{self.bin_dir}:/usr/bin:/bin",
                        events_dir=self.events, fake_log=self.fake_log, fake_rc=1,
                    )
                    self.assertEqual(result.returncode, 2, result.stderr)
                    self.assertIn("unknown hook mode", result.stderr)
                    self.assertIn("gate, brief or notify", result.stderr)
                    # Fail closed means: uv was never started for the bogus mode.
                    self.assertEqual(self.uv_calls(), [])


class TestRunnerExitCodes(FakeUvCase):
    """R5 + R12: the exit-code matrix, each mode, each shell; 13 maps to the
    py-floor fix and every other crash names a terminal command."""

    # rc -> (gate exit, stderr needle for gate; None means "stderr empty").
    # Per hooks.md 2.2: gate blocks with 2; brief and notify exit 0 on every
    # failure (brief prints the down briefing, notify is silent).
    EXPECT = {
        0: (0, None),
        1: (2, "could not run (exit 1)"),
        2: (2, "uv python install 3.12"),  # find fails: the floor is missing
        10: (2, None),  # a rule blocked: the runner explained on stderr
        13: (2, "uv python install 3.12"),
        137: (2, "could not run (exit 137)"),
    }

    def test_exit_code_matrix_per_mode_and_shell(self) -> None:
        for interpreter in INTERPRETERS:
            for mode in GATE_MODES:
                for rc, (gate_rc, needle) in self.EXPECT.items():
                    with self.subTest(shell=interpreter, mode=mode, rc=rc):
                        result = self.run_hook(mode, interpreter, fake_rc=rc, fake_find_rc=2)
                        # The design's skeleton maps uniformly: 0 -> 0, 10 -> 2
                        # (a block), anything else through fail() -- whose brief
                        # and notify modes absorb the failure with exit 0.
                        if rc == 0:
                            want_rc = 0
                        elif rc == 10:
                            want_rc = 2
                        else:
                            want_rc = 2 if mode == "gate" else 0
                        self.assertEqual(
                            result.returncode, want_rc,
                            f"rc {rc} mode {mode} shell {interpreter}: stderr={result.stderr!r}",
                        )
                        if mode == "gate":
                            if needle is None:
                                if rc == 0:
                                    self.assertEqual(result.stderr, "")
                            else:
                                self.assertIn(needle, result.stderr)
                        elif mode == "brief":
                            if rc in (0, 10):
                                self.assertEqual(result.stdout, "")
                            else:
                                briefing = json.loads(result.stdout)
                                context = briefing["hookSpecificOutput"]["additionalContext"]
                                if needle:
                                    self.assertIn(needle, context)
                                # R12: no doubled full stop at the join.
                                self.assertNotIn(".;", context)
                        else:  # notify
                            self.assertEqual(result.stderr, "")
                            self.assertEqual(result.stdout, "")

    def test_uv_exit_2_with_working_interpreter_names_three_commands(self) -> None:
        """R3: uv unusable (cache, old flag) gets its own message, not the floor one."""
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter):
                result = self.run_hook("gate", interpreter, fake_rc=2, fake_find_rc=0)
                self.assertEqual(result.returncode, 2)
                self.assertIn("uv could not run (exit 2)", result.stderr)
                for needle in ("uv --version", "uv cache dir", "uv self update", "brew upgrade uv"):
                    self.assertIn(needle, result.stderr, "hooks.md 2.9 names the upgrade commands")

    def test_killed_runner_maps_to_launch_rc(self) -> None:
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter):
                result = self.run_hook("gate", interpreter, fake_rc=137, fake_find_rc=2)
                self.assertEqual(result.returncode, 2)
                self.assertIn("exit 137", result.stderr)
                self.assertIn("uv --version", result.stderr)

    def test_runner_block_10_is_silent_but_maps_to_two(self) -> None:
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter):
                result = self.run_hook("gate", interpreter, fake_rc=10)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stderr, "")

    def test_floor_exit_13_names_the_install_command(self) -> None:
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter):
                result = self.run_hook("gate", interpreter, fake_rc=13)
                self.assertEqual(result.returncode, 2)
                self.assertIn("uv python install 3.12", result.stderr)
                self.assertIn("[py_floor]", result.stderr)


class TestCliEntry(FakeUvCase):
    """R4 + R5: the cli entry discriminates uv's exit 2 from vsdd's own usage exit 2."""

    def test_success_passthrough(self) -> None:
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter):
                result = self.run_cli(interpreter, "--version", fake_rc=0)
                self.assertEqual(result.returncode, 0)
                self.assertTrue(self.uv_calls(), "the fake uv must have been invoked")

    def test_usage_exit_2_passes_through_silent(self) -> None:
        """vsdd's own usage error (rc 2, interpreter fine) must not be mislabelled."""
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter):
                result = self.run_cli(interpreter, fake_rc=2, fake_find_rc=0)
                self.assertEqual(result.returncode, 2)
                self.assertNotIn("uv python install", result.stderr)

    def test_no_python_names_the_install_command(self) -> None:
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter):
                result = self.run_cli(interpreter, fake_rc=2, fake_find_rc=2)
                self.assertEqual(result.returncode, 2)
                self.assertIn("uv python install 3.12", result.stderr)

    def test_vsdd_exit_1_passes_through_silent(self) -> None:
        """SL1-03: vsdd's own negative answer (rc 1) is the code, not a launch failure."""
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter):
                result = self.run_cli(interpreter, fake_rc=1)
                self.assertEqual(result.returncode, 1)
                self.assertEqual(result.stdout, "")
                self.assertEqual(result.stderr, "")

    def test_crash_propagates_the_code(self) -> None:
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter):
                result = self.run_cli(interpreter, fake_rc=3)
                self.assertEqual(result.returncode, 3)
                self.assertIn("exit 3", result.stderr)
                self.assertIn("uv --version", result.stderr)

    def test_uv_missing_via_seam(self) -> None:
        """The seam is honoured by the cli entry too, and documented in the launcher."""
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter):
                result = launch(
                    interpreter, ["cli"], self.project,
                    path="/usr/bin:/bin", selftest_no_uv=True,
                )
                self.assertEqual(result.returncode, 2)
                self.assertIn("uv is not installed", result.stderr)
                self.assertIn("brew install uv", result.stderr)

    def test_uv_found_off_path_in_home_local_bin(self) -> None:
        """A desktop-launcher PATH still finds uv in ~/.local/bin (hooks.md 2.2)."""
        home = self.root / "home"
        make_fake_uv(home / ".local" / "bin", self.fake_log)
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter):
                self.fake_log.write_text("")
                result = launch(
                    interpreter, ["cli"], self.project,
                    path="/usr/bin:/bin", home=home,
                    events_dir=self.events, fake_log=self.fake_log, fake_rc=0,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn(f"uv0={home}/.local/bin/uv", "\n".join(self.uv_calls()))

    def test_hook_entry_also_found_off_path(self) -> None:
        home = self.root / "home2"
        make_fake_uv(home / ".local" / "bin", self.fake_log)
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter):
                self.fake_log.write_text("")
                result = launch(
                    interpreter, ["hook", "gate"], self.project,
                    path="/usr/bin:/bin", home=home,
                    events_dir=self.events, fake_log=self.fake_log, fake_rc=0,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn(f"uv0={home}/.local/bin/uv", "\n".join(self.uv_calls()))


class TestInertAndDisabled(FakeUvCase):
    """hooks.md 2.14 inert tests: exit 0, silent, uv never invoked, nothing created."""

    def test_inert_outside_factory_project(self) -> None:
        plain = make_project(self.root, factory=False, legacy=False)
        for interpreter in INTERPRETERS:
            for mode in GATE_MODES:
                with self.subTest(shell=interpreter, mode=mode):
                    self.fake_log.write_text("")
                    result = launch(
                        interpreter, ["hook", mode], plain,
                        path=f"{self.bin_dir}:/usr/bin:/bin",
                        events_dir=self.events, fake_log=self.fake_log, fake_rc=0,
                    )
                    self.assertEqual(result.returncode, 0)
                    self.assertEqual(result.stdout, "")
                    self.assertEqual(result.stderr, "")
                    self.assertEqual(self.uv_calls(), [], "the inert path must not invoke uv")
                    self.assertFalse((plain / ".factory" / "local").exists(),
                                     "the inert path must not create an event log")

    def test_hooks_off_writes_disabled_event_once_per_call(self) -> None:
        for interpreter in INTERPRETERS:
            with self.subTest(shell=interpreter):
                shutil.rmtree(self.events, ignore_errors=True)  # one log per subtest
                result = self.run_hook("brief", interpreter, hooks_off=True, fake_rc=0)
                self.assertEqual(result.returncode, 0)
                self.assertIn("VSDD_FACTORY_HOOKS=off", result.stdout)
                events = self.event_lines()
                self.assertEqual(len(events), 1, f"one runner.disabled per call: {events}")
                self.assertEqual(events[0]["type"], "runner.disabled")
                self.assertEqual(events[0]["severity"], "info")


class TestEventLogSchema(FakeUvCase):
    """R8: every appended line carries the fields hooks.md 2.12 requires —
    validated against the shipped EVENT_SCHEMA, not merely by hand-picked
    field checks (CATCHUP-R14: the hand-built dict proved nothing about the
    line the launcher actually writes)."""

    REQUIRED = {"v", "ts", "type", "severity", "session_id"}

    @classmethod
    def setUpClass(cls) -> None:
        sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "plugins" / "vsdd-factory" / "py"))
        from vsdd.events import EVENT_SCHEMA, _TS_READ_RE  # noqa: PLC0415

        cls.event_schema = EVENT_SCHEMA
        cls.ts_pattern = _TS_READ_RE

    def validate_against_schema(self, event: dict) -> None:
        from vsdd.schema import validate  # noqa: PLC0415

        errors = validate(event, self.event_schema)
        self.assertEqual([str(e) for e in errors], [], f"launcher line fails EVENT_SCHEMA: {event}")

    def test_launch_failed_line_matches_schema(self) -> None:
        result = self.run_hook("gate", "sh", fake_rc=1, fake_find_rc=2)
        self.assertEqual(result.returncode, 2)
        lines = self.event_lines()
        self.assertTrue(lines, "a launch failure must append an event")
        for event in lines:
            self.validate_against_schema(event)
            self.assertRegex(event["ts"], self.ts_pattern.pattern + r"$")
            self.assertIsNone(event["session_id"], "a launcher-written line carries a null session_id")
            for field in self.REQUIRED:
                self.assertIn(field, event, f"event missing {field!r}: {event}")
            self.assertEqual(event["v"], 1)
            self.assertRegex(event["ts"], r"T\d{2}:\d{2}:\d{2}Z$")
            self.assertIn(".", event["type"], f"type must be dotted: {event['type']!r}")
            self.assertIn(event["severity"], {"info", "warn", "block", "error"})
            self.assertEqual(event["type"], "runner.launch_failed")
            self.assertIn("code", event)
            self.assertIn("mode", event)

    def test_hooks_off_line_matches_schema(self) -> None:
        """The other launcher-written line (runner.disabled) validates too."""
        result = self.run_hook("brief", "sh", hooks_off=True, fake_rc=0)
        self.assertEqual(result.returncode, 0)
        events = self.event_lines()
        self.assertTrue(events)
        for event in events:
            self.validate_against_schema(event)
            self.assertEqual(event["type"], "runner.disabled")

    def test_vsdd_events_dir_redirects_the_log(self) -> None:
        redirected = self.root / "redirected"
        launch(
            "sh", ["hook", "gate"], self.project,
            path=f"{self.bin_dir}:/usr/bin:/bin", events_dir=redirected,
            fake_log=self.fake_log, fake_rc=1, fake_find_rc=2,
        )
        files = list(redirected.glob("*.jsonl"))
        self.assertEqual(len(files), 1, "VSDD_EVENTS_DIR must redirect the event log")


def launch_env(project: Path) -> dict:
    import os

    env = dict(os.environ)
    env["CLAUDE_PROJECT_DIR"] = str(project)
    return env


class TestBareNameInvocation(unittest.TestCase):
    """SL1-13 (CATCHUP-R14): invoking the launcher by bare name (`sh launch.sh`, no
    slash in $0) must still find its sources — the `here=.` branch of the case guard.
    Run with the working directory set to py/, the only place a bare name resolves."""

    def test_bare_name_invocation_from_the_py_directory(self) -> None:
        py_dir = Path(__file__).resolve().parents[2] / "plugins" / "vsdd-factory" / "py"
        with tempfile.TemporaryDirectory(prefix="vsdd-bare-") as td:
            project = make_project(Path(td))
            for interpreter in INTERPRETERS:
                with self.subTest(shell=interpreter):
                    result = subprocess.run(
                        [interpreter, "launch.sh", "cli", "--version"],
                        cwd=py_dir,
                        env=launch_env(project),
                        capture_output=True,
                        text=True,
                        timeout=60,
                        check=False,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertIn("vsdd 1.0.0", result.stdout)


class TestReadOnlyCache(unittest.TestCase):
    """R3 with the real uv: a broken cache must be reported as 'uv cannot run',
    not misreported as 'no Python' -- the probe isolates itself from the cache."""

    def test_readonly_cache_names_uv_unusable(self) -> None:
        if shutil.which("uv") is None:
            self.skipTest("uv is not installed")
        with tempfile.TemporaryDirectory(prefix="vsdd-rocache-") as td:
            root = Path(td)
            project = make_project(root)
            ro_cache = root / "ro-cache"
            ro_cache.mkdir()
            ro_cache.chmod(0o555)
            # Restore before the TemporaryDirectory cleanup runs (it removes the tree).
            self.addCleanup(lambda: ro_cache.exists() and ro_cache.chmod(0o755))
            for interpreter in INTERPRETERS:
                with self.subTest(shell=interpreter):
                    result = launch(
                        interpreter, ["hook", "gate"], project,
                        path="/usr/bin:/bin:/usr/local/bin:/opt/homebrew/bin",
                        extra_env={"UV_CACHE_DIR": str(ro_cache)},
                    )
                    self.assertEqual(result.returncode, 2, result.stderr)
                    self.assertIn("uv could not run (exit 2)", result.stderr)
                    self.assertIn("uv cache dir", result.stderr)
                    self.assertNotIn("uv python install", result.stderr)


class TestCleanEnvStripsForgedRoot(unittest.TestCase):
    """R21: clean_env clears a forged VSDD_FACTORY_ROOT when no project was found."""

    def test_forged_root_is_gone_from_the_child_environment(self) -> None:
        with tempfile.TemporaryDirectory(prefix="vsdd-forged-") as td:
            root = Path(td)
            plain = make_project(root, factory=False, legacy=False)
            bin_dir = root / "bin"
            fake_log = root / "uv-calls.log"
            make_fake_uv(bin_dir, fake_log)
            result = launch(
                "sh", ["cli"], plain,
                path=f"{bin_dir}:/usr/bin:/bin",
                events_dir=root / "events", fake_log=fake_log, fake_rc=0,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            calls = fake_log.read_text()
            self.assertIn("| root=", calls)
            # No root was found (plain project), so the child must not see one.
            self.assertIn("| root=\n", calls)
            forged = [line for line in calls.splitlines() if "root=/forged" in line]
            self.assertEqual(forged, [], f"a forged root leaked to the child: {calls!r}")

    def test_real_root_still_forwarded(self) -> None:
        with tempfile.TemporaryDirectory(prefix="vsdd-real-") as td:
            root = Path(td)
            project = make_project(root)
            bin_dir = root / "bin"
            fake_log = root / "uv-calls.log"
            make_fake_uv(bin_dir, fake_log)
            result = launch(
                "sh", ["cli"], project,
                path=f"{bin_dir}:/usr/bin:/bin",
                events_dir=root / "events", fake_log=fake_log, fake_rc=0,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertIn(f"root={project}", fake_log.read_text())


if __name__ == "__main__":
    unittest.main()
