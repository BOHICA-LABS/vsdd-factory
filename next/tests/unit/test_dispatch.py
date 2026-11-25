"""The dispatcher's contract (state-core.md 3.1): registration by file alone,
resolution, exit codes, relayed output, the person check and the raw grammar."""

from __future__ import annotations

import io
import json
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

from vsdd.cli import dispatch
from vsdd.cli.dispatch import Command, Reply, Refuse

from ._repo import make_project, run_init


def run(argv, **kw) -> tuple[int, str, str]:
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(argv, stdout=out, stderr=err, **kw)
    return code, out.getvalue(), err.getvalue()


def make_pkg(monkeypatch, tmp_path: Path, name: str, files: dict[str, str]) -> str:
    pkg = tmp_path / name
    pkg.mkdir(exist_ok=True)
    (pkg / "__init__.py").write_text("", encoding="utf-8")
    for fname, content in files.items():
        (pkg / fname).write_text(content, encoding="utf-8")
    monkeypatch.syspath_prepend(str(tmp_path))
    return name


def echo_command(words, needs="none", **kw):
    call = words
    body = (
        "def run(inv, args):\n"
        "    payload = {\n"
        f"        'called': list({call!r}),\n"
        "        'wait': inv.wait,\n"
        "        'argv': list(inv.argv),\n"
        "        'retry': inv.retry_command(),\n"
        "        'start': str(inv.start),\n"
        "    }\n"
        "    tokens = getattr(args, 'tokens', None)\n"
        "    if tokens is not None:\n"
        "        payload['tokens'] = list(tokens)\n"
        "    return Reply(data=payload)\n"
    )
    fields = ", ".join(
        f"{key}={value}"
        for key, value in (
            ("needs", repr(needs)),
            ("raw", repr(kw.get("raw", False))),
            ("output", repr(kw.get("output"))),
            (
                "failure",
                "{'status': 'blocked', 'next': {}}"
                if kw.get("output")
                else "{}",
            ),
        )
    )
    return (
        "from vsdd.cli.dispatch import Command, Reply\n"
        "def _cfg(parser):\n"
        "    parser.add_argument('--target')\n"
        "    parser.add_argument('rest', nargs='*')\n"
        + body
        + f"COMMANDS = (Command(words={words!r}, summary='echo', run=run, "
        f"configure={'_cfg' if kw.get('configure', True) else 'None'}, {fields}),)\n"
    )


def test_registers_by_file_alone(monkeypatch, tmp_path):
    pkg = make_pkg(monkeypatch, tmp_path, "sd_reg", {"cmd_alpha.py": echo_command(("alpha",))})
    code, out, _ = run(["alpha", "go"], packages=(pkg,))
    assert (code, json.loads(out)["called"]) == (0, ["alpha"])


def test_longest_words_win(monkeypatch, tmp_path):
    pkg = make_pkg(
        monkeypatch,
        tmp_path,
        "sd_long",
        {
            "cmd_beta.py": echo_command(("beta",)),
            "cmd_beta_deep.py": echo_command(("beta", "deep")),
        },
    )
    packages = ("vsdd.state", pkg)
    code, out, _ = run(["beta", "deep"], packages=packages)
    assert json.loads(out)["called"] == ["beta", "deep"]
    code, out, _ = run(["beta", "shallow"], packages=packages)
    assert json.loads(out)["called"] == ["beta"]


def test_duplicate_words_exit_2_naming_both_modules(monkeypatch, tmp_path):
    pkg = make_pkg(
        monkeypatch,
        tmp_path,
        "sd_dup",
        {
            "cmd_gamma.py": echo_command(("gamma",)),
            "cmd_gamma_other.py": echo_command(("gamma",)),
        },
    )
    code, _, err = run(["gamma"], packages=(pkg,))
    assert code == 2
    assert "cmd_gamma" in err and "cmd_gamma_other" in err


def test_broken_command_module_is_loud(monkeypatch, tmp_path):
    pkg = make_pkg(
        monkeypatch,
        tmp_path,
        "sd_broken",
        {
            "cmd_delta.py": "this is ( not python\n",
            "cmd_whole.py": echo_command(("whole",)),
        },
    )
    code, _, err = run(["delta"], packages=(pkg,))
    assert code == 2
    assert "cmd_delta.py" in err
    # Under --json the failure is one object on stdout, whatever went wrong.
    code, out, err = run(["delta", "--json"], packages=(pkg,))
    assert code == 2
    payload = json.loads(out)
    assert payload["ok"] is False
    assert "cmd_delta.py" in payload["error"]
    # Other families still run.
    code, out, _ = run(["whole"], packages=(pkg,))
    assert code == 0


def test_unknown_command_exits_2(monkeypatch, tmp_path):
    pkg = make_pkg(monkeypatch, tmp_path, "sd_unknown", {"cmd_here.py": echo_command(("here",))})
    for name in ("flush", "stale", "job", "step", "nonesuch"):
        code, _, err = run([name], packages=(pkg,))
        assert (code, "no command matches" in err) == (2, True), name
        # Under --json even a command that never resolves prints its object.
        code, out, _ = run([name, "--json"], packages=(pkg,))
        assert code == 2, name
        payload = json.loads(out)
        assert payload["ok"] is False and payload["error"], name


def test_C_names_the_project(monkeypatch, tmp_path):
    root = make_project(tmp_path / "c")
    elsewhere = tmp_path / "elsewhere"
    elsewhere.mkdir()
    code, out, err = run(["-C", str(root), "config", "--json"], cwd=str(elsewhere))
    assert (code, json.loads(out)["ok"]) == (0, True)
    # Later -C is refused.
    code, _, err = run(["config", "--json", "-C", str(root)], cwd=str(root))
    assert code == 2
    # From a story worktree the -C directory still names the project.
    worktree = root / ".worktrees" / "S-9.01"
    subprocess.run(
        ["git", "worktree", "add", "-q", "-b", "story/9.01", str(worktree), "develop"],
        cwd=root, check=True, capture_output=True,
    )
    code, out, _ = run(["-C", str(worktree), "config", "--json"])
    assert (code, json.loads(out)["ok"]) == (0, True)


def test_needs_levels(monkeypatch, tmp_path):
    pkg = make_pkg(
        monkeypatch,
        tmp_path,
        "sd_needs",
        {
            "cmd_free.py": echo_command(("free",), needs="none"),
            "cmd_needinit.py": echo_command(("needinit",), needs="initialised"),
            "cmd_needmount.py": echo_command(("needmount",), needs="mounted"),
        },
    )
    bare = tmp_path / "bare"
    bare.mkdir()
    code, out, err = run(["needinit"], cwd=str(bare), packages=(pkg,))
    assert code == 2 and "/vsdd-factory:setup" in err

    root = tmp_path / "handmade"
    (root / ".factory").mkdir(parents=True)
    (root / ".factory" / "factory.toml").write_text("schema = 1\n", encoding="utf-8")
    code, out, err = run(["needinit"], cwd=str(root), packages=(pkg,))
    assert (code, json.loads(out)["ok"]) == (0, True)
    # A hand-made project has no mount: the mounted command refuses.
    code, out, err = run(["needmount"], cwd=str(root), packages=(pkg,))
    assert code == 2 and "/vsdd-factory:status --fix" in err


def test_json_is_one_object_with_ok(monkeypatch, tmp_path):
    pkg = make_pkg(monkeypatch, tmp_path, "sd_json", {"cmd_jay.py": echo_command(("jay",))})
    code, out, _ = run(["jay", "--json"], packages=(pkg,))
    lines = out.splitlines()
    assert code == 0 and len(lines) == 1
    payload = json.loads(lines[0])
    assert payload["ok"] is True


def test_crash_exits_2_never_1(monkeypatch, tmp_path):
    crashing = (
        "from vsdd.cli.dispatch import Command, Reply\n"
        "def run(inv, args):\n    raise RuntimeError('exploded')\n"
        "COMMANDS = (Command(words=('boom',), summary='', run=run, needs='none'),)\n"
    )
    pkg = make_pkg(monkeypatch, tmp_path, "sd_boom", {"cmd_boom.py": crashing})
    code, out, err = run(["boom", "--json"], packages=(pkg,))
    assert code == 2
    assert "internal error: RuntimeError" in err
    assert json.loads(out) == {"ok": False, "error": "internal error: RuntimeError: exploded"}


def test_invalid_config_exits_2_naming_key(monkeypatch, tmp_path):
    from ._repo import set_keys

    root = make_project(tmp_path / "badcfg")
    set_keys(root, "limits", agents_per_run=0)
    code, out, err = run(["-C", str(root), "config", "--json"])
    assert code == 2
    assert "agents_per_run" in err
    assert json.loads(out)["ok"] is False


def test_wait_and_json_on_every_command(monkeypatch, tmp_path):
    pkg = make_pkg(monkeypatch, tmp_path, "sd_wait", {"cmd_slow.py": echo_command(("slow",))})
    code, out, _ = run(["slow", "--json", "--wait", "7"], packages=(pkg,))
    payload = json.loads(out)
    assert (code, payload["wait"]) == (0, 7)
    code, out, _ = run(["slow", "--json"], packages=(pkg,))
    assert json.loads(out)["wait"] is None


def test_decode_text():
    assert dispatch.decode_text("plain") == "plain"
    assert dispatch.decode_text("a%20b") == "a b"
    assert dispatch.decode_text("caf%C3%A9") == "café"
    with pytest.raises(Refuse):
        dispatch.decode_text("a%zz")


def test_version_and_no_command():
    code, out, _ = run(["--version"])
    assert code == 0 and out.startswith("vsdd ")
    code, _, err = run([])
    assert code == 2 and "no command" in err
    # An unreadable manifest is an environment error: exit 2, not a traceback.
    original = dispatch._PLUGIN_ROOT
    dispatch._PLUGIN_ROOT = original / "nowhere"
    try:
        code, out, err = run(["--version"])
    finally:
        dispatch._PLUGIN_ROOT = original
    assert code == 2 and "manifest" in err


def test_help_lists_commands(monkeypatch, tmp_path):
    pkg = make_pkg(monkeypatch, tmp_path, "sd_help", {"cmd_halp.py": echo_command(("halp",))})
    code, out, _ = run(["--help"], packages=(pkg,))
    assert code == 0
    assert "halp" in out
    code, out, _ = run(["-h"], packages=(pkg,))
    assert code == 0 and "halp" in out
    # The shipped commands appear with their summaries.
    code, out, _ = run(["--help"])
    assert code == 0
    for words in ("init", "config", "validate", "path"):
        assert words in out


def test_relative_C_resolves_against_the_caller_directory(monkeypatch, tmp_path):
    """A relative -C leaves an absolute start, resolved against the caller's
    directory (the cwd argument, else the process directory); symlinks are
    untouched (finding S3R-state-core-16)."""
    root = make_project(tmp_path / "rel")
    nested = tmp_path / "nested"
    nested.mkdir()
    # "../rel/proj" from nested names the project: start is made absolute.
    code, out, err = run(["-C", "../rel/proj", "config", "--json"], cwd=str(nested))
    assert (code, json.loads(out)["ok"]) == (0, True), err
    # An absolute -C start is still accepted; the resolved value is absolute.
    pkg = make_pkg(monkeypatch, tmp_path, "sd_start", {"cmd_where.py": echo_command(("where",))})
    code, out, _ = run(["-C", ".", "where"], cwd=str(nested), packages=(pkg,))
    payload = json.loads(out)
    assert payload["start"] == str(nested)


def test_main_from_a_deleted_working_directory(monkeypatch, tmp_path):
    """Even from a deleted cwd, an absolute -C works and a bare call exits 2
    with a message, not a traceback (finding S3R-state-core-7)."""
    import os

    root = make_project(tmp_path / "gone")
    gone = tmp_path / "vanishing"
    gone.mkdir()
    old = os.getcwd()
    os.chdir(gone)
    gone.rmdir()
    try:
        code, out, err = run(["-C", str(root), "config", "--json"])
        assert (code, json.loads(out)["ok"]) == (0, True)
        code, out, err = run(["config", "--json"])
        assert code == 2
        assert "working directory" in err
        assert "Traceback" not in err
    finally:
        os.chdir(old)


def test_error_labels_use_command_words(monkeypatch, tmp_path):
    """Once resolved, the stderr label names the command, not the arguments,
    and the un-initialised refusal names the detected condition
    (finding S3R-state-core-20)."""
    pkg = make_pkg(monkeypatch, tmp_path, "sd_label", {"cmd_lab.py": echo_command(("lab",))})
    code, out, err = run(["lab", "positional", "--nonsense"], packages=(pkg,))
    assert code == 2
    assert err.startswith("vsdd lab:")
    assert "positional" not in err.split("\n")[0]
    # The un-initialised refusal names the condition (detect of the start).
    needer = make_pkg(
        monkeypatch,
        tmp_path,
        "sd_label2",
        {"cmd_needer.py": echo_command(("needer",), needs="initialised")},
    )
    bare = tmp_path / "bare-label"
    bare.mkdir()
    code, out, err = run(["needer"], cwd=str(bare), packages=(needer,))
    assert code == 2
    assert "condition 'none'" in err
    assert "/vsdd-factory:setup" in err


def test_registration_rejects_malformed_commands(monkeypatch, tmp_path):
    """Empty words, or a first word that is not the module's family, are
    RegistrationErrors (finding S3R-state-core-18)."""
    from vsdd.cli.dispatch import RegistrationError

    wrong_family = make_pkg(
        monkeypatch,
        tmp_path,
        "sd_family",
        {"cmd_stray.py": echo_command(("elsewhere",))},
    )
    with pytest.raises(RegistrationError) as raised:
        dispatch.commands(packages=(wrong_family,))
    assert "stray" in str(raised.value)
    assert "elsewhere" in str(raised.value)

    empty = make_pkg(
        monkeypatch,
        tmp_path,
        "sd_emptywords",
        {
            "cmd_blank.py": (
                "from vsdd.cli.dispatch import Command, Reply\n"
                "def run(inv, args):\n    return Reply()\n"
                "COMMANDS = (Command(words=(), summary='', run=run),)\n"
            )
        },
    )
    with pytest.raises(RegistrationError) as raised:
        dispatch.commands(packages=(empty,))
    assert "at least one word" in str(raised.value)


def test_hyphenated_first_word_registers_by_its_module_stem(monkeypatch, tmp_path):
    """A first word with a hyphen is registered by the module stem that spells
    it with '-' written '_' (plan 3.1: cmd_lock_release.py holds lock-release;
    finding S3R-state-core-C1-2)."""
    pkg = make_pkg(
        monkeypatch,
        tmp_path,
        "sd_hyphen",
        {"cmd_lock_release.py": echo_command(("lock-release",))},
    )
    table = dispatch.commands(packages=(pkg,))
    assert ("lock-release",) in table
    code, out, _ = run(["lock-release", "now"], packages=(pkg,))
    assert (code, json.loads(out)["called"]) == (0, ["lock-release"])


def test_broken_command_package_is_loud(monkeypatch, tmp_path):
    """A package whose __init__ fails to import is a named error, never a
    silent 'no command matches' (finding S3R-state-core-27)."""
    pkg = make_pkg(
        monkeypatch,
        tmp_path,
        "sd_badpkg",
        {"__init__.py": "raise ImportError('the package is broke')\n"},
    )
    code, out, err = run(["anything"], packages=(pkg,))
    assert code == 2
    assert "failed to import" in err and "the package is broke" in err
    assert "no command matches" not in err
    # Under --json: one object on stdout.
    code, out, _ = run(["anything", "--json"], packages=(pkg,))
    assert code == 2
    assert json.loads(out)["ok"] is False


def test_require_person(monkeypatch, tmp_path):
    body = (
        "from vsdd.cli.dispatch import Command, Reply\n"
        "def run(inv, args):\n"
        "    inv.require_person('testing')\n"
        "    return Reply(data={'who': 'person'})\n"
        "COMMANDS = (Command(words=('person',), summary='', run=run, needs='none'),)\n"
    )
    pkg = make_pkg(monkeypatch, tmp_path, "sd_person", {"cmd_person.py": body})
    code, out, err = run(["person"], packages=(pkg,), stdin_tty=False)
    assert code == 2 and "terminal" in err
    code, out, err = run(["person"], packages=(pkg,), stdin_tty=True)
    assert (code, json.loads(out)["who"]) == (0, "person")
    code, out, err = run(
        ["person"], packages=(pkg,), stdin_tty=True, environ={"VSDD_CALLER": "agent:x"}
    )
    assert code == 2 and "agent" in err


def test_raw_command_gets_tokens(monkeypatch, tmp_path):
    pkg = make_pkg(
        monkeypatch, tmp_path, "sd_raw", {"cmd_deecide.py": echo_command(("deecide",), raw=True)}
    )
    argv = ["deecide", "--json", "--wait", "3", "story", "--", "--json", "why"]
    code, out, _ = run(argv, packages=(pkg,))
    payload = json.loads(out)
    assert payload["called"] == ["deecide"]
    assert payload["wait"] == 3
    # The exact tokens survive, in order: the flag before -- is taken, the
    # flag after it is a plain token (finding S3R-state-core-13).
    assert payload["tokens"] == ["story", "--", "--json", "why"]
    # --json before the separator is the flag; after it, a plain token.
    out_lines = out.strip().splitlines()
    assert len(out_lines) == 1


def test_retry_command(monkeypatch, tmp_path):
    pkg = make_pkg(monkeypatch, tmp_path, "sd_retry", {"cmd_again.py": echo_command(("again",))})
    code, out, _ = run(["again", "one", "two"], packages=(pkg,))
    payload = json.loads(out)
    assert payload["argv"] == ["again", "one", "two"]
    assert " ".join(payload["argv"]) == "again one two"
    # The retry text is the whole call, shlex-joined from the real argv.
    assert payload["retry"] == "vsdd again one two"


def test_relayed_failure_prints_whole_object(monkeypatch, tmp_path):
    from vsdd.schema import validate

    schema = json.loads(
        (dispatch._PLUGIN_ROOT / "schemas" / "survey.schema.json").read_text(encoding="utf-8")
    )
    pkg = make_pkg(
        monkeypatch,
        tmp_path,
        "sd_relay",
        {
            "cmd_relay.py": (
                "from vsdd.cli.dispatch import Command, Reply, Refuse\n"
                "from vsdd.config import ConfigError\n"
                "def run(inv, args):\n"
                "    if args.target == 'refuse':\n"
                "        raise Refuse('the clerk said no')\n"
                "    if args.target == 'crash':\n"
                "        raise RuntimeError('mid-crash')\n"
                "    if args.target == 'config':\n"
                "        raise ConfigError('[limits]: agents_per_run must be at least 1')\n"
                "    return Reply(ok=False, code=1, data={'status': 'blocked'})\n"
                "def cfg(parser):\n    parser.add_argument('target')\n"
                "COMMANDS = (Command(words=('relay',), summary='', run=run, configure=cfg,"
                " needs='none', output='survey', failure={'status': 'blocked'}),)\n"
            )
        },
    )
    for target in ("refuse", "crash", "config"):
        code, out, err = run(["relay", target, "--json"], packages=(pkg,))
        assert code == 2
        payload = json.loads(out.splitlines()[-1])
        # The relayed object is schema-valid, not merely key-complete.
        assert validate(payload, schema) == [], (target, payload)
        assert payload["ok"] is False and payload["status"] == "blocked", target
        if target == "refuse":
            assert payload["error"] == "the clerk said no"
    # On exit 0 and 1 the command prints every field itself.
    code, out, err = run(["relay", "blocked", "--json"], packages=(pkg,))
    assert code == 1
    assert json.loads(out) == {"ok": False, "status": "blocked"}
    # A usage error has the same shape: --json is read from the raw tokens.
    code, out, err = run(["relay", "--json", "--nonsense"], packages=(pkg,))
    assert code == 2
    payload = json.loads(out.splitlines()[-1])
    assert validate(payload, schema) == []
    assert payload["ok"] is False and payload["status"] == "blocked"


def test_shipped_commands_register_cleanly():
    from vsdd.cli.dispatch import commands, empty_of
    from vsdd.schema import validate

    table = commands()
    for words in (("init",), ("config",), ("validate",), ("path",)):
        assert words in table, words
    schemas: dict[str, dict] = {}
    for words, command in table.items():
        if not command.output:
            continue
        name = command.output
        if name not in schemas:
            schemas[name] = json.loads(
                (dispatch._PLUGIN_ROOT / "schemas" / f"{name}.schema.json").read_text(
                    encoding="utf-8"
                )
            )
        # The whole object an exit 2 prints must pass the command's schema,
        # not merely key-subset empty_of (finding S3R-state-core-13).
        payload = {**empty_of(name), **dict(command.failure), "ok": False, "error": "refused"}
        problems = validate(payload, schemas[name])
        assert problems == [], (words, problems)


def test_import_error_exits_2(tmp_path):
    source = dispatch._PLUGIN_ROOT / "py"
    copy = tmp_path / "py"
    subprocess.run(["cp", "-R", str(source), str(copy)], check=True)
    (copy / "vsdd" / "cli" / "dispatch.py").write_text("this is ( broken\n", encoding="utf-8")
    # uv and the interpreter live where they live: the launcher must find
    # them whatever the machine layout (finding S3R-state-core-28).
    path_parts = ["/usr/bin", "/bin"]
    for tool in (shutil.which("uv"), sys.executable):
        if tool:
            parent = str(Path(tool).parent)
            if parent not in path_parts:
                path_parts.insert(0, parent)
    proc = subprocess.run(
        ["sh", str(copy / "launch.sh"), "cli", "--version"],
        capture_output=True,
        text=True,
        env={"PATH": ":".join(path_parts), "HOME": str(tmp_path)},
        cwd=str(tmp_path),  # no .venv in sight: uv must not need one
    )
    assert proc.returncode == 2
    assert "vsdd:" in proc.stderr
