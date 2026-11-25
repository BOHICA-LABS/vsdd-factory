"""The caller marker and the command rewrite (plan 3.5; [[record:caller-marker]]).

The runner only writes markers: reading, deleting and pruning them is
records' ``vsdd.state.callers.caller``. For a ``vsdd`` call (HD8) that no rule
blocked, the runner writes
``<root>/.factory/local/hooks/calls/<key>.json`` — written to the runner's own
temporary directory ``<root>/.factory/local/hooks/tmp/<key>.json.tmp`` first
and renamed into place, so a reader never sees half of it; the temporary file
never sits in ``calls/``, where the reader's prune (``*.json`` only) would
never lift a leftover — and returns the call's whole ``tool_input`` with
``command`` set to ``VSDD_CALLER=<key> `` followed by the command as typed.
Two units meet at the marker file, so its schema ships (HD13,
``schemas/caller-marker.schema.json``).
"""

from __future__ import annotations

import json
import os
import re
import secrets
from datetime import datetime, timezone
from pathlib import Path

from .model import Rewrite, _home

__all__ = [
    "CALLER_PREFIX",
    "MARKER_SCHEMA",
    "MarkerUnwritable",
    "forged",
    "is_vsdd_call",
    "marker_rewrite",
    "marker_path",
]

CALLER_PREFIX = "VSDD_CALLER="

MARKER_SCHEMA = Path(__file__).resolve().parents[2] / "schemas" / "caller-marker.schema.json"


class MarkerUnwritable(OSError):
    """The marker could not be written; the call blocks (HD9), because a call
    let through without its marker would report dead hooks — the wrong fault."""


def forged(command_text: str) -> bool:
    """HD9: a Bash command that sets ``VSDD_CALLER`` itself is blocked, which
    otherwise waits for ``gate-guard`` (H6)."""
    return CALLER_PREFIX in command_text


# --- the allow-list (ORCH-2) ---------------------------------------------------
#
# Approval is decided on the raw command text first: the parser's "not opaque"
# is never by itself a reason to approve. Four review rounds each found a shell
# construct that read as one plain ``vsdd`` call while running something else
# (a quote nested in ``${...}``, bash/zsh ``$[..]``, ANSI-C ``$'...'``, bash's
# ``${name@P}``), and a deny-list over shell syntax cannot be shown complete
# across sh, bash, dash and zsh. So the whole text must match this positive
# grammar, once, instead; the parsed HD8 shape below is only the second gate.
#
# The text is: optional leading and trailing spaces or tabs; the program, which
# is ``vsdd`` or a path of safe characters ending in ``/vsdd`` that holds no
# ``=`` (a first word holding one is an assignment, and the shell runs the next
# word, not the path) and begins with a letter, a digit, ``_``, ``.`` or ``/``
# (step3-hook-runner-fix-4-C2-1); then zero or more arguments, each introduced
# by one or more spaces or tabs. An argument is
# a bare word of safe characters that does not start with ``=`` (nor with a
# tilde, which is no safe character), or a bare-word prefix that is empty or
# ends with ``=`` followed by one single-quoted string, or one single-quoted
# string alone. A single-quoted string holds no single quote and no character
# below U+0020 or U+007F, and is followed by a space, a tab or the end of the
# text. What each excluded character would otherwise smuggle past the parse:
#   ``"``  opens a double-quoted span this grammar does not read
#   ``$``  parameter, arithmetic and command expansion: ``$(...)``,
#          ``${...}``, ``$[..]``, ``$'...'`` disagree across the shells
#   `` ` `` command substitution
#   ``\``  escapes and line continuations move where words and quotes end
#   ``;`` ``&`` ``|``  joiners: a second command would ride the approval
#   ``<`` ``>``  redirects and here-documents
#   ``(`` ``)``  subshells and command substitutions
#   ``{`` ``}``  brace expansion, and the ``${...}`` delimiters
#   ``[`` ``]``  globs, ``$[..]`` arithmetic, array subscripts
#   ``*`` ``?``  globs: what they match is decided at run time, not by the parse
#   ``~``  tilde expansion
#   ``#``  a comment, truncating the command in every shell
#   ``!``  history expansion, ``${!x}`` indirection, bash's ``${x@P}``, which
#          expands a value as a prompt string and so runs any ``$(...)`` in it
# A newline is a joiner; a carriage return is an ordinary word character that
# hides joiners and comments (step3-hook-runner-fix-3-C1-2). Nothing below
# U+0020 nor U+007F may sit inside a single-quoted string either.
_SAFE = r"A-Za-z0-9_@%+=:,./-"  # the safe bare-word characters; "=" only mid-word
_QUOTED = r"'[^'\x00-\x1f\x7f]*'"  # no quote inside, no control character, no DEL
_CALL_TEXT_RE = re.compile(
    # The program's path holds no "=": a first word holding one is an
    # assignment, and the shell runs the next word (step3-hook-runner-fix-4-C1-1).
    # It begins with a letter, a digit, "_", "." or "/":
    # step3-hook-runner-fix-4-C2-1 — bash, macOS /bin/sh and zsh read a first
    # word beginning with "%" as a job spec and run the fg builtin, not the
    # path, and the interpreter of a "#!/bin/sh" script invoked through a word
    # beginning with "-" or "+" reads that word as options, so such a call is
    # approved and can never work.
    # The character before the final "/vsdd" may be the word's only one: the
    # bare "/vsdd" is a path of safe characters that begins with "/" and holds
    # no "=", an ordinary absolute path in sh, bash, dash and zsh, exactly
    # like "//vsdd" (step3-hook-runner-fix-6-C1-1).
    rf"[ \t]*(?:vsdd|(?:[A-Za-z0-9_./][A-Za-z0-9_@%+:,./-]*)?/vsdd)"
    rf"(?:[ \t]+(?:(?:(?!=)[{_SAFE}]*=)?{_QUOTED}|(?!=)[{_SAFE}]+))*[ \t]*"
)


def is_vsdd_call(command) -> bool:
    """HD8's shape AND the ORCH-2 allow-list: the whole raw command text must
    match the grammar above, and the command must be exactly one segment, not
    opaque, with no redirect and no leading assignment, whose program's last
    path part is ``vsdd``. The decision sent with the rewrite approves the
    whole command, so it may hold nothing else — and the parser's ``not
    opaque`` is never by itself a reason to approve. Any command that fails
    either gate passes unmarked and undecided, as HD8 says for any other
    command that runs ``vsdd``."""
    if not _CALL_TEXT_RE.fullmatch(command.text):
        return False
    if command.opaque or len(command.segments) != 1:
        return False
    segment = command.segments[0]
    if segment.opaque or segment.redirects or segment.assignments:
        return False
    if not segment.program:
        return False
    return os.path.basename(segment.program) == "vsdd"


def marker_rewrite(ctx, command) -> Rewrite | None:
    """Write the marker and return the rewrite for one ``vsdd`` call, or
    ``None`` when the call passes as typed, unmarked (no governing project
    found from the directory the call names, else the payload ``cwd``)."""
    segment = command.segments[0]
    root = _project_root(segment.where, ctx)
    if root is None:
        return None
    key = secrets.token_hex(16)
    write_marker(ctx, key, root)
    original = str(ctx.tool_input.get("command", ""))
    tool_input = dict(ctx.tool_input)
    tool_input["command"] = f"{CALLER_PREFIX}{key} {original}"
    return Rewrite(tool_input=tool_input, reason=f"the caller marker {key} of this vsdd call")


def _project_root(directory: str, ctx) -> Path | None:
    from .. import project as _project

    if not directory:
        return None
    return _project.find_project_root(directory, home=_home(ctx.env))


def marker_path(root: Path | str, key: str) -> Path:
    return Path(root) / ".factory" / "local" / "hooks" / "calls" / f"{key}.json"


def _temporary_path(root: Path | str, key: str) -> Path:
    """The temporary file of one marker, under ``<hooks>/tmp/`` — never in
    ``calls/``, so a kill before the rename leaves nothing the reader's prune
    (``*.json`` in ``calls/``) would trip over."""
    return (
        Path(root) / ".factory" / "local" / "hooks" / "tmp" / f"{key}.json.tmp"
    )


def write_marker(ctx, key: str, root: Path | str) -> Path:
    """Write one marker atomically; the fields and types of
    ``caller-marker.schema.json``, all required. ``agent_id`` and
    ``agent_type`` are ``""`` for the main session; ``ts`` is the event log's
    millisecond UTC form. The file is staged under ``<hooks>/tmp/`` and
    renamed into ``calls/`` (one filesystem: the rename is atomic)."""
    marker = {
        "v": 1,
        "key": key,
        "session_id": ctx.session_id,
        "agent_id": ctx.agent_id,
        "agent_type": ctx.agent_type,
        "cwd": ctx.cwd,
        "ts": datetime.now(timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z"),
    }
    path = marker_path(root, key)
    temporary = _temporary_path(root, key)
    try:
        temporary.parent.mkdir(parents=True, exist_ok=True)
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary.write_text(json.dumps(marker, ensure_ascii=False), encoding="utf-8")
        os.replace(temporary, path)
    except OSError as exc:
        try:
            temporary.unlink(missing_ok=True)
        except OSError:
            pass
        raise MarkerUnwritable(f"cannot write the caller marker at {path}: {exc}") from exc
    return path
