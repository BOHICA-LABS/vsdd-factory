"""The ``vsdd env`` commands on the command line (plan step-3/jobs-b.md).

The handlers are ``test_env.py``'s; these tests exercise the wrapping the
dispatcher (state-core.md 3.1) gives them — in process through ``main``, and
once through the real launcher. The loud-failure test is
``test_stop_without_marker``: a tool call without a marker stops nothing,
exits 1 and names the single-segment form that works (D16).
"""

from __future__ import annotations

import fcntl
import io
import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

from tests.unit._repo import make_project, set_keys
from vsdd.cli import dispatch
from vsdd.gates import env
from vsdd.gates.jobs import job_key, root_of, stop_products, store_of

RUN = "echo {port} > started.txt; echo $VSDD_PORT >> started.txt; sleep 30"

LAUNCH_SH = dispatch._PLUGIN_ROOT / "py" / "launch.sh"


def record_path(root: Path, subject: str) -> Path:
    key = job_key("env", subject, "")
    return root_of(root) / ".factory" / "local" / "jobs" / f"{key}.json"


def write_marker(root: Path, key: str, session: str, agent: str) -> str:
    """The marker file the hook runner would have written for a marked call.

    Reading it deletes it, so every marked call needs a marker of its own.
    """
    calls = root / ".factory" / "local" / "hooks" / "calls"
    calls.mkdir(parents=True, exist_ok=True)
    marker = {
        "v": 1,
        "key": key,
        "session_id": session,
        "agent_id": agent,
        "agent_type": "vsdd-factory:clerk",
        "cwd": str(root),
        "ts": "2026-01-01T00:00:00.000Z",
    }
    (calls / f"{key}.json").write_text(json.dumps(marker), encoding="utf-8")
    return key


def run_main(argv, *, stdin_tty=True, environ):
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(argv, stdout=out, stderr=err, stdin_tty=stdin_tty, environ=environ)
    return code, out.getvalue(), err.getvalue()


@pytest.fixture
def project(tmp_path, monkeypatch):
    root = make_project(tmp_path)
    set_keys(root, "commands", run=RUN)
    (root / ".worktrees" / "S-1.01").mkdir(parents=True, exist_ok=True)
    monkeypatch.setattr(env, "SETTLE_SECONDS", 0.3)  # the CLI test runs unpatched
    yield root
    stop_products(root, grace=1.5)


# --- registration ------------------------------------------------------------


def test_env_commands_register():
    table = dispatch.commands()
    for words in (("env", "start"), ("env", "stop")):
        command = table[words]
        assert command.needs == "initialised"  # stoppable on a broken mount
        assert command.output is None  # not relayed by a script


# --- usage errors, exit 2 ------------------------------------------------------


def test_start_bad_subject_exits_2(project, tmp_path):
    for subject in ("../evil", "x" * 100):
        code, out, err = run_main(
            ["-C", str(project), "env", "start", "--subject", subject, "--json"],
            environ={"HOME": str(tmp_path)},
        )
        assert code == 2, (out, err)
        payload = json.loads(out)
        assert payload["ok"] is False
        assert "subject" in payload["error"]
        assert "vsdd" in err and "subject" in err
        # A usage error, not a crash: the ValueError of a malformed subject is
        # converted to Refuse, not left to the dispatcher's internal-error path.
        assert "internal error" not in err
        assert "internal error" not in payload["error"]
    # A malformed subject starts nothing and leaves nothing in the job store
    # (S3R-jobs-b-5): whichever subject the call names, no file may appear.
    made = [path for pattern in ("*.json", "*.lock", "*.log") for path in store_of(project).glob(pattern)]
    assert made == []


def test_cli_env_stop_rejects_subject(project, tmp_path):
    """AU-S2-12: ``env stop`` has the one syntax; a subject is a usage error."""
    code, out, err = run_main(
        ["-C", str(project), "env", "stop", "--subject", "S-1.01", "--json"],
        environ={"HOME": str(tmp_path)},
    )
    assert code == 2
    assert "--subject" in err
    payload = json.loads(out)
    assert payload["ok"] is False
    assert "--subject" in payload["error"]


def test_env_outside_a_project_exits_2(tmp_path):
    plain = tmp_path / "plain"
    plain.mkdir()
    code, out, err = run_main(
        ["-C", str(plain), "env", "start", "--subject", "S-1.01", "--json"],
        environ={"HOME": str(tmp_path)},
    )
    assert code == 2
    assert "initialised" in err
    payload = json.loads(out)
    assert payload["ok"] is False
    code, out, err = run_main(
        ["-C", str(plain), "env", "stop", "--json"],
        environ={"HOME": str(tmp_path)},
    )
    assert code == 2
    assert "initialised" in err


# --- stop without a marker (D16) ------------------------------------------------


def test_stop_without_marker(project, tmp_path):
    started, code = env.start(project, "S-1.01", None)
    assert code == 0, started
    environ = {"HOME": str(tmp_path)}  # no VSDD_CALLER: no marker
    # From a tool call nothing stops and the error names the single-segment
    # form that works; the product lives on (the loud failure of D16).
    code, out, _err = run_main(
        ["-C", str(project), "env", "stop", "--json"], stdin_tty=False, environ=environ
    )
    assert code == 1
    payload = json.loads(out)
    assert payload["ok"] is False
    assert payload["stopped"] == []
    assert "vsdd -C <dir> env stop" in payload["error"]
    assert record_path(project, "S-1.01").exists()
    # From a person's terminal every product stops.
    code, out, _err = run_main(
        ["-C", str(project), "env", "stop", "--json"], stdin_tty=True, environ=environ
    )
    assert code == 0
    payload = json.loads(out)
    assert payload["ok"] is True
    assert [entry["subject"] for entry in payload["stopped"]] == ["S-1.01"]
    assert not record_path(project, "S-1.01").exists()


def test_stop_marked_call_without_marker_stops_nothing(project, tmp_path):
    """A call carrying VSDD_CALLER is a tool call wherever its marker can be
    read or not (S3R-jobs-b-2): with the key set but the marker pruned, under
    another -C root or already read, a terminal stdin still stops nothing —
    the refinement of D3 in cmd_env's ``_run_stop``."""
    started, code = env.start(project, "S-1.01", None)
    assert code == 0, started
    environ = {"HOME": str(tmp_path), "VSDD_CALLER": "cmdenv0010"}  # no marker file
    code, out, _err = run_main(
        ["-C", str(project), "env", "stop", "--json"], stdin_tty=True, environ=environ
    )
    assert code == 1
    payload = json.loads(out)
    assert payload["ok"] is False
    assert payload["stopped"] == []
    assert record_path(project, "S-1.01").exists()


# --- through the real launcher --------------------------------------------------

requires_uv = pytest.mark.skipif(shutil.which("uv") is None, reason="uv is not installed")


def _launcher_environ(caller_key: str | None) -> dict[str, str]:
    """The process environment of a launched ``vsdd``: the real one, minus
    this session's ``VSDD_*`` and ``UV_*`` names, with uv's directory first
    and, when the call is marked, the ``VSDD_CALLER`` the rewrite carries."""
    env_vars = {
        key: value
        for key, value in os.environ.items()
        if not key.startswith("VSDD_") and not key.startswith("UV_")
    }
    path_parts = ["/usr/bin", "/bin"]
    uv = shutil.which("uv")
    if uv:
        parent = str(Path(uv).parent)
        if parent not in path_parts:
            path_parts.insert(0, parent)
    tail = env_vars.get("PATH", "")
    env_vars["PATH"] = ":".join([*path_parts, tail]) if tail else ":".join(path_parts)
    if caller_key is not None:
        env_vars["VSDD_CALLER"] = caller_key
    return env_vars


def _launch_cli(env_vars, project, cwd, *args):
    return subprocess.run(
        ["sh", str(LAUNCH_SH), "cli", "-C", str(project), *args],
        capture_output=True,
        text=True,
        stdin=subprocess.DEVNULL,  # a launched vsdd is never on a terminal
        env=env_vars,
        cwd=str(cwd),
        timeout=180,
    )


@requires_uv
def test_cli_env_commands(project, tmp_path):
    env_vars = _launcher_environ("cmdenv0001")
    write_marker(project, "cmdenv0001", "sess-cli-1", "agent-cli-1")
    # P3SEAM-20: --wait is global; a command that is not a job accepts it and
    # ignores it, so the closing command of a workflow carries it unchanged.
    proc = _launch_cli(
        env_vars, project, tmp_path,
        "env", "start", "--subject", "S-1.01", "--json", "--wait", "540",
    )
    assert proc.returncode == 0, (proc.stdout, proc.stderr)
    lines = proc.stdout.strip().splitlines()
    assert len(lines) == 1
    payload = json.loads(lines[0])
    assert payload["ok"] is True
    assert set(payload) == {"ok", "url", "port", "pid"}
    assert payload["url"] == f"http://127.0.0.1:{payload['port']}"
    record = json.loads(record_path(project, "S-1.01").read_text(encoding="utf-8"))
    assert record["session_id"] == "sess-cli-1"
    assert record["agent_id"] == "agent-cli-1"
    # Another agent's marked stop stops nothing and leaves the product running:
    # the passing stop below must be the marker's doing, never the terminal.
    write_marker(project, "cmdenv0009", "sess-other", "agent-other")
    proc = _launch_cli(
        _launcher_environ("cmdenv0009"), project, tmp_path, "env", "stop", "--json"
    )
    assert proc.returncode == 1, (proc.stdout, proc.stderr)
    others = json.loads(proc.stdout.strip())
    assert others["ok"] is False
    assert others["stopped"] == []
    assert record_path(project, "S-1.01").exists()
    # The start's read consumed the first marker; the stop writes and reads
    # its own, marked as the same agent and session (D13: no argument).
    write_marker(project, "cmdenv0002", "sess-cli-1", "agent-cli-1")
    proc = _launch_cli(
        _launcher_environ("cmdenv0002"), project, tmp_path, "env", "stop", "--json"
    )
    assert proc.returncode == 0, (proc.stdout, proc.stderr)
    stopped = json.loads(proc.stdout.strip())
    assert stopped["ok"] is True
    assert [entry["subject"] for entry in stopped["stopped"]] == ["S-1.01"]
    assert stopped["stopped"][0]["pid"] == payload["pid"]
    assert not record_path(project, "S-1.01").exists()


# --- the refusals the spec's failure-case list must name (S3R-jobs-b-C1-1) -------


def _grab_lock(path: Path):
    """Context manager taking the flock at ``path`` for this test process.

    ``flock`` locks are per open file description, so this holder conflicts
    with every descriptor ``env.start`` opens for itself, as another process
    would. The store directory is made first: the refusals below are reached
    before ``start`` has created anything.
    """
    path.parent.mkdir(parents=True, exist_ok=True)
    fd = os.open(path, os.O_RDWR | os.O_CREAT, 0o644)

    class _Hold:
        def __enter__(self):
            fcntl.flock(fd, fcntl.LOCK_EX)
            return fd

        def __exit__(self, *_exc):
            fcntl.flock(fd, fcntl.LOCK_UN)
            os.close(fd)
            return False

    return _Hold()


def test_start_refusal_ports_lock_stays_held(project, monkeypatch):
    """ports.lock held past the wait while another start is choosing a port is
    the same negative answer, exit 1 — one of the three refusals the spec's
    failure-case list came out without (S3R-jobs-b-C1-1)."""
    monkeypatch.setattr(env, "_STARTING_WAIT", 0.1)
    with _grab_lock(store_of(project) / "ports.lock"):
        output, code = env.start(project, "S-1.01", None)
    assert code == 1
    assert output["ok"] is False
    assert "ports.lock" in output["error"]
    assert "choosing a port" in output["error"]
    assert output["log"].endswith(f"{job_key('env', 'S-1.01', '')}.log")


def test_start_refusal_product_record_over_cap(project, monkeypatch):
    """A product record over the job record's 4,000-byte cap less the 128
    bytes kept for later writes is the same negative answer, exit 1
    (S3R-jobs-b-C1-1). The cap is shrunk here so any record trips it."""
    monkeypatch.setattr(env, "RECORD_CAP", 200)
    monkeypatch.setattr(env, "RECORD_HEADROOM", 0)
    output, code = env.start(project, "S-1.01", None)
    assert code == 1
    assert output["ok"] is False
    assert "the product record would be" in output["error"]
    assert "200-byte limit" in output["error"]
    key = job_key("env", "S-1.01", "")
    assert not (store_of(project) / f"{key}.json").exists()


def test_start_refusal_another_start_still_settling(project, monkeypatch):
    """A holder the wait can resolve nothing for - here one that leaves no
    record behind at all - is answered with the still-settling refusal after
    the wait, not with a hang or a crash (S3R-jobs-b-C1-1)."""
    monkeypatch.setattr(env, "_STARTING_WAIT", 0.2)
    key = job_key("env", "S-1.01", "")
    with _grab_lock(store_of(project) / f"{key}.lock"):
        output, code = env.start(project, "S-1.01", None)
    assert code == 1
    assert output["ok"] is False
    assert "still settling" in output["error"]
    assert output["log"].endswith(f"{key}.log")


def test_start_refusals_are_all_named_in_the_spec():
    """The closed failure-case list in env-start.md names every refusal
    ``env.start`` answers with. It came out three cases short once
    (S3R-jobs-b-C1-1); this is the check that would have caught it. The three
    drives above are the code side of the same list."""
    spec = Path(__file__).resolve().parents[3] / "rewrite" / "spec" / "commands" / "env-start.md"
    text = " ".join(spec.read_text(encoding="utf-8").split())  # the prose is hard-wrapped
    for case in (
        "the worktree for the subject does not exist",
        "`commands.run` is empty",
        "no port is free in 28100 to 28199",
        "exits on three ports",
        "product lock stays held",
        "`ports.lock` stays held while another start is choosing a port",
        "4,000-byte cap less the 128 bytes kept for later writes",
        "still settling after the five-second wait",
    ):
        assert case in text, f"env-start.md does not name the refusal: {case}"
