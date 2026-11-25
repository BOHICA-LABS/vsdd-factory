"""The one dispatcher every ``vsdd`` command registers with (state-core.md 3.1).

A command is a module named ``cmd_<family>[_<x>].py`` in ``vsdd.state``,
``vsdd.gates``, ``vsdd.delivery`` or ``vsdd.cli``, holding
``COMMANDS = (Command(...), ...)``. The dispatcher resolves the command's
family from the leading tokens, lists the family's modules in the four
packages without importing the others, imports only those, and hands the rest
of the tokens to argparse.

Exit codes (topic:command-conventions): 0 success or "yes", 1 the negative
answer, 2 a usage or environment error. A crash exits 2, never 1: 1 is the
checked answer a workflow reads.
"""

from __future__ import annotations

import argparse
import importlib
import importlib.util
import json
import os
import shlex
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Mapping

__all__ = [
    "COMMAND_PACKAGES",
    "Command",
    "Invocation",
    "Refuse",
    "RegistrationError",
    "Reply",
    "commands",
    "decode_text",
    "empty_of",
    "main",
]

#: The packages whose ``cmd_*`` modules register commands. A missing package
#: is skipped, so a unit adds its commands without editing this list.
COMMAND_PACKAGES = ("vsdd.state", "vsdd.gates", "vsdd.delivery", "vsdd.cli")

_PLUGIN_ROOT = Path(__file__).resolve().parents[3]


class Refuse(Exception):
    """A usage or environment error; the only way to exit 2."""


class RegistrationError(Exception):
    """Two commands claim the same words, or a ``cmd_`` module is malformed."""


@dataclass(frozen=True)
class Command:
    """One registered command; see state-core.md 3.1 for every field."""

    words: tuple[str, ...]  # ("gate", "red"); words[0] is the family
    summary: str  # one line, for --help
    run: Callable[["Invocation", argparse.Namespace], "Reply"]
    configure: Callable[[argparse.ArgumentParser], None] | None = None
    needs: str = "initialised"  # "none" | "initialised" | "mounted"
    raw: bool = False  # True: nothing is parsed; run reads args.tokens
    output: str | None = None  # a relayed command's schema name, e.g. "survey"
    failure: Mapping[str, object] = field(default_factory=dict)


@dataclass
class Reply:
    """What a command returns: everything the dispatcher prints."""

    ok: bool = True
    data: dict = field(default_factory=dict)  # merged after "ok"; must not hold "ok"
    text: str | None = None  # printed without --json; default: the JSON, indented
    code: int | None = None  # 0 or 1 only; default 0 when ok, else 1

    @classmethod
    def of(cls, output: dict, code: int | None = None) -> "Reply":
        """Build from an object that already holds ``"ok"``."""
        data = dict(output)
        ok = bool(data.pop("ok", True))
        return cls(ok=ok, data=data, code=code)


@dataclass
class Invocation:
    """Everything one run of a command sees (state-core.md 3.1)."""

    words: tuple[str, ...]
    argv: tuple[str, ...]  # every token after the program name, as received
    start: Path  # absolute: the -C directory, else the working directory
    root: Path | None  # vsdd.project.find_project_root(start, home=$HOME)
    json: bool
    wait: int | None  # --wait; None when absent
    caller_key: str | None  # $VSDD_CALLER, set by the hook's rewrite
    stdin_tty: bool  # standard input is a terminal
    plugin_root: Path
    environ: Mapping[str, str]

    _config: object = field(default=None, init=False, repr=False)
    _condition: str | None = field(default=None, init=False, repr=False)

    @property
    def state_dir(self) -> Path | None:
        return self.root / ".factory" if self.root is not None else None

    @property
    def config(self):
        """The loaded project configuration, once per invocation.

        A ``ConfigError`` escapes: the dispatcher reports it like a Refuse
        and exits 2, naming the offending key.
        """
        if self._config is None:
            from ..config import load

            self._config = load(self.root / ".factory" / "factory.toml")
        return self._config

    @property
    def condition(self) -> str:
        """``detect`` on the project root, once per invocation."""
        if self._condition is None:
            from ..state.mount import detect

            self._condition = detect(self.root if self.root is not None else self.start)
        return self._condition

    def retry_command(self) -> str:
        """The whole call as text, for a pending answer the clerk re-runs."""
        return "vsdd " + shlex.join(self.argv)

    def require_person(self, what: str) -> None:
        """The tool's one person-only check (decision 2): Refuse otherwise."""
        if self.caller_key is not None:
            raise Refuse(f"{what}: the caller is an agent ({self.caller_key}); only a person may run it")
        if not self.stdin_tty:
            raise Refuse(f"{what}: standard input is not a terminal; only a person may run it")


# ---------------------------------------------------------------- schemas ---

def _schema_path(name: str) -> Path:
    return _PLUGIN_ROOT / "schemas" / f"{name}.schema.json"


def empty_of(name: str) -> dict:
    """Every required key of the named schema with its empty value (the
    prelude's ``emptyOf``): ``0``, ``false``, ``[]``, ``{}``, ``""``."""
    with _schema_path(name).open(encoding="utf-8") as handle:
        schema = json.load(handle)
    if isinstance(schema.get("anyOf"), list) and schema["anyOf"]:
        schema = schema["anyOf"][0]
    out: dict = {}

    def fill(properties: dict, required: list, target: dict) -> None:
        for key in required:
            spec = properties.get(key) or {}
            kind = spec.get("type")
            if kind in ("integer", "number"):
                target[key] = 0
            elif kind == "boolean":
                target[key] = False
            elif kind == "array":
                target[key] = []
            elif kind == "object":
                nested: dict = {}
                fill(spec.get("properties") or {}, spec.get("required") or [], nested)
                target[key] = nested
            else:
                target[key] = ""

    fill(schema.get("properties") or {}, schema.get("required") or [], out)
    return out


# ------------------------------------------------------------- resolution ---

def _prefix_of(words: tuple[str, ...], candidates: tuple[str, ...]) -> bool:
    return len(words) <= len(candidates) and candidates[: len(words)] == words


def _package_dirs(packages: tuple[str, ...]) -> list[tuple[str, Path]]:
    """(package, directory) for each package; a missing one skips.

    Only a package that does not exist at all is skipped: a package that is
    there but fails to import is a loud error, never a silent lack of
    commands.
    """
    dirs: list[tuple[str, Path]] = []
    for package in packages:
        try:
            found = importlib.util.find_spec(package)
        except (ImportError, ValueError):
            found = None
        if found is None:
            continue
        try:
            module = importlib.import_module(package)
        except ImportError as exc:
            raise Refuse(f"command package {package} failed to import: {exc}") from exc
        base = Path(str(getattr(module, "__file__", ""))).parent
        if base.is_dir():
            dirs.append((package, base))
    return dirs


def _family_modules(base: Path, family: str) -> list[str]:
    """The ``cmd_<family>[_<x>]`` module stems of one package, unimported."""
    stems: list[str] = []
    for path in sorted(base.glob(f"cmd_{family}*.py")):
        if path.stem == f"cmd_{family}" or path.stem.startswith(f"cmd_{family}_"):
            stems.append(path.stem)
    return stems


def _load_command(package: str, stem: str) -> list[Command]:
    """Import one ``cmd_`` module and return its commands, loudly."""
    module = importlib.import_module(f"{package}.{stem}")
    declared = getattr(module, "COMMANDS", None)
    if declared is None:
        raise RegistrationError(f"{package}.{stem} has no COMMANDS")
    try:
        entries = tuple(declared)
    except TypeError as exc:
        raise RegistrationError(f"{package}.{stem}: COMMANDS is not a tuple: {exc}") from exc
    family = stem[len("cmd_") :]
    for entry in entries:
        if not isinstance(entry, Command):
            raise RegistrationError(
                f"{package}.{stem}: COMMANDS holds a {type(entry).__name__}, not a Command"
            )
        if not entry.words:
            raise RegistrationError(f"{package}.{stem}: a Command needs at least one word")
        # The module stem spells the first word with '-' written '_':
        # cmd_lock_release.py registers lock-release; a longer stem may refine
        # the family's single-word command (cmd_survey_extra.py holds survey).
        # Splitting at the first '_' would lose a hyphenated word
        # (finding S3R-state-core-C1-2).
        word = entry.words[0].replace("-", "_")
        if word != family and not family.startswith(word + "_"):
            raise RegistrationError(
                f"{package}.{stem}: the first word {entry.words[0]!r} does not match "
                f"the module's family {family.replace('_', '-')!r}"
            )
    return list(entries)


def _resolve(packages: tuple[str, ...], candidates: tuple[str, ...]) -> Command:
    """The command whose ``words`` is the longest prefix of the candidates."""
    family = candidates[0].replace("-", "_")
    found: dict[tuple[str, ...], tuple[str, Command]] = {}
    for package, base in _package_dirs(packages):
        for stem in _family_modules(base, family):
            try:
                entries = _load_command(package, stem)
            except RegistrationError:
                raise
            except Exception as exc:  # a broken module is a loud usage error
                raise Refuse(
                    f"command module {package}.{stem} failed to import: "
                    f"{type(exc).__name__}: {exc}"
                ) from exc
            for command in entries:
                if command.words[0].replace("-", "_") != family or not _prefix_of(
                    command.words, candidates
                ):
                    continue
                if command.words in found:
                    other = found[command.words][0]
                    raise Refuse(
                        f"{' '.join(command.words)} is registered by both {other} "
                        f"and {package}.{stem}"
                    )
                found[command.words] = (f"{package}.{stem}", command)
    if not found:
        raise Refuse(f"no command matches {' '.join(candidates)!r}")
    return max(found.values(), key=lambda pair: len(pair[1].words))[1]


def commands(packages: tuple[str, ...] = COMMAND_PACKAGES) -> dict[tuple[str, ...], Command]:
    """Import every ``cmd_`` module; words -> Command. Loud on duplicates and
    malformed modules: ``--help``, B6's lints and the tests use this."""
    table: dict[tuple[str, ...], Command] = {}
    for package, base in _package_dirs(packages):
        for path in sorted(base.glob("cmd_*.py")):
            for command in _load_command(package, path.stem):
                if command.words in table:
                    raise RegistrationError(
                        f"{' '.join(command.words)} is registered by more than one module"
                    )
                table[command.words] = command
    return table


# ------------------------------------------------------------ text codec ---

_HEX = set("0123456789abcdefABCDEF")


def decode_text(value: str) -> str:
    """Decode the prelude's ``enc()``: percent-encoded UTF-8 free text.

    ``urllib`` is banned from the shipped package, so this is the tool's own
    decoder. A malformed escape is a ``Refuse``, not a guess.
    """
    if "%" not in value:
        return value
    out: list[str] = []
    buffer = bytearray()
    chars = list(value)
    i = 0
    while i < len(chars):
        char = chars[i]
        if char == "%":
            if i + 2 >= len(chars) or chars[i + 1] not in _HEX or chars[i + 2] not in _HEX:
                raise Refuse(f"malformed percent escape in {value!r} at offset {i}")
            buffer.append(int(value[i + 1 : i + 3], 16))
            i += 3
            continue
        if buffer:
            out.append(_decode(buffer, value))
            buffer = bytearray()
        out.append(char)
        i += 1
    if buffer:
        out.append(_decode(buffer, value))
    return "".join(out)


def _decode(buffer: bytearray, original: str) -> str:
    try:
        return buffer.decode("utf-8")
    except UnicodeDecodeError as exc:
        raise Refuse(f"{original!r} is not valid percent-encoded UTF-8: {exc}") from exc


# ----------------------------------------------------------------- output ---

def _json_line(payload: dict) -> str:
    return json.dumps(payload) + "\n"


def _relay_failure(command: Command | None, message: str) -> str | None:
    """The whole object a relayed command prints on an exit 2, or None."""
    if command is None or not command.output:
        return None
    payload = {**empty_of(command.output), **dict(command.failure)}
    payload["ok"] = False
    payload["error"] = message
    return _json_line(payload)


class _Parser(argparse.ArgumentParser):
    """Usage errors are Refuse, so a relayed command still prints its object."""

    def error(self, message: str):  # noqa: A003 - argparse's own name
        raise Refuse(message)


def _wait_type(text: str) -> int:
    try:
        seconds = int(text)
    except ValueError:
        raise argparse.ArgumentTypeError(
            f"--wait needs a non-negative integer, got {text!r}"
        ) from None
    if seconds < 0:
        raise argparse.ArgumentTypeError(f"--wait needs a non-negative integer, got {seconds}")
    return seconds


def _split_raw(tokens: list[str]) -> tuple[bool, int | None, list[str]]:
    """Take ``--json`` and ``--wait <n>`` out of the tokens before the first
    lone ``--``; the rest, in order, stays (a raw command's grammar)."""
    json_flag = False
    wait: int | None = None
    rest: list[str] = []
    index = 0
    while index < len(tokens):
        token = tokens[index]
        if token == "--":
            rest.extend(tokens[index:])
            break
        if token == "--json":
            json_flag = True
            index += 1
            continue
        if token == "--wait":
            if index + 1 >= len(tokens) or not tokens[index + 1].isdigit():
                raise Refuse("--wait needs a non-negative integer")
            wait = int(tokens[index + 1])
            index += 2
            continue
        rest.append(token)
        index += 1
    return json_flag, wait, rest


def _plugin_version(plugin_root: Path) -> str:
    manifest = plugin_root / ".claude-plugin" / "plugin.json"
    try:
        with manifest.open(encoding="utf-8") as handle:
            return str(json.load(handle)["version"])
    except (OSError, KeyError, json.JSONDecodeError) as exc:
        raise Refuse(f"cannot read the plugin manifest at {manifest}: {exc}") from exc


def _start_dir(c_value: str | None, cwd: str | None) -> Path:
    """The absolute start directory: the ``-C`` value when given, else the
    working directory. A relative ``-C`` resolves against ``cwd`` when the
    caller names one, else against the process directory. Symlinks are not
    resolved, and the process cwd is consulted only when it is needed.
    """
    try:
        if c_value is not None:
            candidate = Path(c_value)
            if not candidate.is_absolute():
                base = Path(cwd) if cwd is not None else Path.cwd()
                candidate = base / candidate
            return Path(os.path.abspath(candidate))
        if cwd is not None:
            return Path(os.path.abspath(cwd))
        return Path.cwd()
    except OSError as exc:  # a deleted working directory, say
        raise Refuse(f"cannot reach the working directory: {exc}") from exc


def _needs_check(invocation: Invocation, command: Command) -> None:
    if command.needs == "none":
        return
    if invocation.root is None:
        raise Refuse(
            f"this is not an initialised vsdd-factory project (condition "
            f"{invocation.condition!r}); run /vsdd-factory:setup"
        )
    if command.needs == "mounted" and invocation.condition != "ready":
        raise Refuse(
            "the state directory is not mounted (condition "
            f"{invocation.condition!r}); run /vsdd-factory:status --fix"
        )


def _help_text(packages: tuple[str, ...]) -> str:
    table = commands(packages)
    rows = [(" ".join(words), command.summary) for words, command in sorted(table.items())]
    width = max((len(name) for name, _ in rows), default=0)
    lines = ["vsdd <command> [--json] [--wait <seconds>]", "", "commands:"]
    lines.extend(f"  {name:<{width}}  {summary}" for name, summary in rows)
    return "\n".join(lines) + "\n"


def main(
    argv: list[str],
    *,
    cwd: str | None = None,
    environ: Mapping[str, str] | None = None,
    stdout=None,
    stderr=None,
    stdin_tty: bool | None = None,
    packages: tuple[str, ...] = COMMAND_PACKAGES,
) -> int:
    """One whole run of ``vsdd``; returns the process exit code."""
    environ = os.environ if environ is None else environ
    argv = list(argv)
    stdout = sys.stdout if stdout is None else stdout
    stderr = sys.stderr if stderr is None else stderr

    tokens = list(argv)
    if tokens and tokens[0] == "-C":
        if len(tokens) < 2:
            print("vsdd: -C needs a directory argument", file=stderr)
            return 2
        c_value = tokens[1]
        tokens = tokens[2:]
    else:
        c_value = None

    # --json is read from the raw tokens before resolution, so even a command
    # that never resolves prints its failure object (3.1's Refuse rule).
    json_requested = "--json" in tokens

    candidates: list[str] = []
    for token in tokens:
        if token.startswith("-"):
            break
        candidates.append(token)

    command: Command | None = None
    try:
        if tokens and tokens[0] in ("--help", "-h") and not candidates:
            stdout.write(_help_text(packages))
            return 0
        if tokens and tokens[0] == "--version":
            print(f"vsdd {_plugin_version(_PLUGIN_ROOT)}", file=stdout)
            return 0
        if not candidates:
            print("vsdd: no command given; try 'vsdd --help'", file=stderr)
            return 2

        start = _start_dir(c_value, cwd)
        command = _resolve(packages, tuple(candidates))
        wait: int | None = None
        rest = tokens[len(command.words) :]  # the command's own words drop off
        if command.raw:
            argv_json, wait, raw_tokens = _split_raw(rest)
            json_requested = argv_json
            args = argparse.Namespace()
            args.tokens = raw_tokens
        else:
            parser = _Parser(prog=f"vsdd {' '.join(command.words)}")
            parser.add_argument("--json", action="store_true", help="print one JSON object")
            parser.add_argument(
                "--wait",
                metavar="SECONDS",
                type=_wait_type,
                default=None,
                help="re-attach to a long command",
            )
            if command.configure is not None:
                command.configure(parser)
            json_requested = "--json" in tokens
            args = parser.parse_args(rest)
            wait = getattr(args, "wait", None)

        from ..project import find_project_root

        root = find_project_root(start, home=environ.get("HOME"))
        invocation = Invocation(
            words=command.words,
            argv=tuple(argv),
            start=start,
            root=root,
            json=json_requested,
            wait=wait,
            caller_key=environ.get("VSDD_CALLER"),
            stdin_tty=bool(stdin_tty) if stdin_tty is not None else sys.stdin.isatty(),
            plugin_root=_PLUGIN_ROOT,
            environ=environ,
        )
        _needs_check(invocation, command)
        reply = command.run(invocation, args)
        if reply.code not in (None, 0, 1):
            raise Refuse(f"command returned exit code {reply.code}; only 0 and 1 exist")
        code = reply.code if reply.code is not None else (0 if reply.ok else 1)
        if json_requested:
            stdout.write(_json_line({"ok": reply.ok, **reply.data}))
        else:
            text = (
                reply.text
                if reply.text is not None
                else json.dumps({"ok": reply.ok, **reply.data}, indent=2)
            )
            stdout.write(text if text.endswith("\n") else text + "\n")
        return code
    except Refuse as exc:
        return _fail(command, str(exc), _labelled_words(command, candidates), json_requested, stdout, stderr)
    except Exception as exc:  # a crash never exits 1
        # A ConfigError from Invocation.config carries its own message (it
        # names the key); any other crash is labelled an internal error.
        # Either way: stderr, exit 2, and with --json one JSON object.
        from ..config import ConfigError

        message = str(exc) if isinstance(exc, ConfigError) else f"internal error: {type(exc).__name__}: {exc}"
        return _fail(command, message, _labelled_words(command, candidates), json_requested, stdout, stderr)


def _labelled_words(command: Command | None, candidates: list[str]) -> tuple[str, ...]:
    """Once the command is resolved, errors are labelled with its words, not
    with whatever positional arguments followed."""
    return command.words if command is not None else tuple(candidates)


def _fail(
    command: Command | None,
    message: str,
    words: tuple[str, ...],
    json_requested: bool,
    stdout,
    stderr,
) -> int:
    label = "vsdd" + (" " + " ".join(words) if words else "")
    print(f"{label}: {message}", file=stderr)
    if json_requested:
        line = _relay_failure(command, message)
        if line is None:
            line = _json_line({"ok": False, "error": message})
        stdout.write(line)
    return 2
