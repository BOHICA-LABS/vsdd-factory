"""The paths one shell segment writes (plan rules.md 3.5).

:func:`write_targets` is the one reader the delivery-order rules share: the
paths a ``Bash`` segment would create, truncate or destroy, so ``tests-first``
judges ``sed -i`` like the edit it stands for. It returns absolute paths and
never raises; a target it cannot resolve is left out, because the gate behind
it (the green gate needs a red record and unchanged frozen files) still holds
for what was missed (decision D5).

Words that expand at run time — ``$``, a backtick — name no guessable file and
are left out; glob words (``*``, ``?`, ``[``) are expanded with :mod:`glob`,
200 paths at most. Relative paths resolve against the segment's ``where``; a
segment with an unknown ``where`` contributes only absolute spellings.
"""

from __future__ import annotations

import glob as _glob
import os

from .shell import Segment, free_text_value_words

__all__ = ["write_targets", "write_unknown"]

_GLOB_MARKS = ("*", "?", "[")
_MAX_EXPANDED = 200

#: Programs whose non-option arguments the call writes into (or, for ``rm``,
#: destroys): the write-scope rule's check 4 list at slice depth.
_WHOLE_ARG_WRITERS = ("tee", "rm", "touch", "truncate")

#: Options of the argument writers that take a separate value; the value is
#: never a path (``truncate -s 0``, ``sed -e 's,x,y,'``, ``touch -r ref``),
#: so it is dropped before the operands are read.
_VALUE_OPTIONS = {
    "truncate": ("-s", "--size"),
    "sed": ("-e", "--expression", "-f", "--file"),
    "perl": ("-e", "-E"),
    "touch": ("-r", "--reference", "-d", "--date", "-t"),
}

#: Programs whose segments a rule cannot always read: an opaque segment
#: whose text names one carries a write whose target the parse does not
#: give (:func:`write_unknown`).
_WRITER_PROGRAMS = frozenset(
    {"sed", "perl", "tee", "rm", "touch", "truncate", "cp", "mv", "ln", "dd"}
)

#: Perl's short options that eat the rest of the word as their value
#: (``-Mmodule``, ``-Fpattern``, ``-Idir``), for the bundled-``-i`` reading.
_PERL_VALUE_LEADERS = frozenset("MFI")

#: The destination-directory options of ``cp`` and kin, long forms included:
#: ``-t <dir>``, ``--target-dir <dir>``, ``--target-dir=<dir>`` and
#: ``--target-directory`` in both spellings (S4R-rules-23).
_TARGET_DIR_OPTIONS = ("-t", "--target-dir", "--target-directory")


def write_targets(seg: Segment) -> tuple[str, ...]:
    """The absolute paths ``seg`` writes, in the order its words name them."""
    try:
        return tuple(_targets(seg))
    except Exception:  # a parse fault passes, never crashes the rule
        return ()


def write_unknown(seg: Segment) -> bool:
    """Whether the segment carries a write the parser cannot read: an opaque
    segment whose text names a writer program, a segment that delegates to a
    writer (``flock lk sed -i …``, ``find . -exec sed -i …``), or a writer
    whose path operand holds an expansion (``sed -i … $F``). Such a write is
    the edit it could be, wherever a protected file could be the target; the
    caller judges it conservatively, since the targets above are the
    readable ones (step4-rules-fix-3-C1-9). A git call's free-text values
    are message text, not writes: a heredoc commit message naming ``rm``
    writes nothing."""
    if seg.program == "git":
        args = list(seg.args)
        values = free_text_value_words(args)
        words = ["git"]
        words.extend(word for index, word in enumerate(args) if index not in values)
        return _unknown_writer_words(words)
    if not seg.opaque:
        program = os.path.basename(seg.program) if seg.program else ""
        if not program:
            return False
        if program in _WRITER_PROGRAMS:
            return _expansion_operand(program, list(seg.args))
        # The program itself writes nothing, but a word of the call may
        # delegate a write to one (step4-rules-fix-3-C1-9).
        return _delegates_write(seg.args)
    text = seg.text or ""
    words = [word.strip("\"'`$(){};,") for word in text.split()]
    return _unknown_writer_words(words)


def _unknown_writer_words(words) -> bool:
    """Whether one of ``words`` names a writer program whole."""
    return any(os.path.basename(word) in _WRITER_PROGRAMS for word in words)


def _delegates_write(args) -> bool:
    return _unknown_writer_words(list(args))


def _expansion_operand(program: str, args: list[str]) -> bool:
    """Whether one of a writer's path operands holds an expansion, so the
    write lands where the parse cannot name (step4-rules-fix-3-C1-9)."""
    trimmed = list(args)
    if program == "perl":
        trimmed = _without_bundled_value(trimmed, "eE")
    trimmed = _without_option_value(trimmed, _VALUE_OPTIONS.get(program, ()))
    return any("$" in word or "`" in word for word in _operands(trimmed))


def _targets(seg: Segment) -> list[str]:
    found: list[str] = []
    where = seg.where or ""
    for redirect in seg.redirects:
        target = _redirect_target(redirect.op, redirect.target)
        if target:
            _add(found, target, where)
    args = list(seg.args)
    # The program by its own name, wherever it is mounted: /usr/bin/git is
    # git for every rule that reads this (S4R-rules-7).
    program = os.path.basename(seg.program) if seg.program else ""
    if program == "dd":
        for arg in args:
            if arg.startswith("of="):
                _add(found, arg[3:], where)
        return _clean(found)
    if program in ("cp", "mv", "ln"):
        _copy_like(found, program, args, where)
        return _clean(found)
    # The script test reads the original arguments: the ``-e <script>`` pairs
    # are removed below, and it must still see them, or sed's files are lost
    # to the script rule that never fires (S4R-rules-1).
    scripted = program == "sed" and _has_script_option(args)
    in_place = (
        program == "sed" and any(_sed_in_place(arg) for arg in args)
    ) or (
        program == "perl" and any(_perl_in_place(arg) for arg in args)
    )
    # An option value that is no path (truncate -s 0) drops before the
    # operands are read. Perl's bundled script option (``-pe``, ``-pE``) eats
    # the word after it, so it is read first: the separate ``-e``/``-E``
    # pairing would otherwise trim the wrong word (S4R-rules-23).
    if program == "perl":
        args = _without_bundled_value(args, "eE")
    args = _without_option_value(args, _VALUE_OPTIONS.get(program, ()))
    operands = _operands(args)
    if in_place or program in _WHOLE_ARG_WRITERS:
        if program == "sed" and not scripted and operands:
            # sed's first operand is the script, not a file (POSIX: the first
            # operand shall be treated as the script).
            operands = operands[1:]
        if program == "perl" and not _perl_scripted(seg.args) and operands:
            # Without -e/-E perl's first operand is the program file, which
            # the in-place edit does not touch (S4R-rules-23).
            operands = operands[1:]
        for operand in operands:
            _add(found, operand, where)
    return _clean(found)


# --- one path word -------------------------------------------------------------


def _add(found: list[str], word: str, where: str) -> None:
    """Resolve one as-typed path against ``where`` and append it, glob words
    expanded. A word that expands at run time, or a relative one with an
    unknown ``where``, names no file and is left out (decision D5)."""
    if not word:
        return
    if "$" in word or "`" in word:
        return
    text = word
    if not text.startswith("/"):
        if not where:
            return
        text = os.path.join(where, text)
    if any(mark in text for mark in _GLOB_MARKS):
        try:
            hits = _glob.glob(text)
        except Exception:  # a pattern the globber refuses is no path
            hits = []
        found.extend(hits[:_MAX_EXPANDED])
        return
    found.append(text)


def _clean(found: list[str]) -> list[str]:
    kept: list[str] = []
    for path in found:
        if path not in kept:
            kept.append(path)
    return kept


# --- redirects -----------------------------------------------------------------


def _redirect_target(op: str, target: str) -> str | None:
    """The target of a redirect that writes to a file: ``>``, ``>>``, ``>|``,
    ``&>``, ``&>>``, ``<>`` and the fd-hugged forms (``2>``), with ``<``'s
    family, an fd duplication (``2>&1``), ``/dev/null`` and ``&N`` left out."""
    if not target:
        return None
    base = op.lstrip("0123456789")
    if base not in (">", ">>", ">|", "&>", "&>>", "<>") and not (
        base == ">&" and not target.isdigit()
    ):
        return None
    if target == "/dev/null" or target.startswith("&"):
        return None
    return target


# --- argument shaping ----------------------------------------------------------


def _operands(args: list[str]) -> list[str]:
    """The non-option arguments: options are dropped, ``--`` ends them, a lone
    ``-`` (stdin/stdout) is not a file."""
    operands: list[str] = []
    end_of_options = False
    for arg in args:
        if end_of_options:
            operands.append(arg)
            continue
        if arg == "--":
            end_of_options = True
            continue
        if arg.startswith("-") and len(arg) > 1:
            continue
        if arg == "-":
            continue
        operands.append(arg)
    return operands


def _option_value(args: list[str], names: tuple[str, ...]) -> str | None:
    """The value of the first ``-t <dir>``-style option: separate
    (``-t <dir>``), fused (``-tdir``) or long with ``=`` (``--target-dir=d``,
    ``--target-directory=d``, S4R-rules-23)."""
    index = 0
    while index < len(args):
        arg = args[index]
        for name in names:
            if arg == name:
                if index + 1 < len(args):
                    return args[index + 1]
                continue
            if name.startswith("--"):
                if arg.startswith(name + "="):
                    return arg[len(name) + 1 :]
            elif (
                not arg.startswith("--")
                and arg.startswith(name)
                and len(arg) > len(name)
            ):
                return arg[len(name) :]
        index += 1
    return None


def _copy_like(found: list[str], program: str, args: list[str], where: str) -> None:
    """``cp``, ``mv`` and ``ln``: the destination is ``-t``'s directory (the
    source's base name there) or the last operand — inside an existing
    directory, the base name of each source beside it. ``mv``'s sources go
    too: they stop being what they were."""
    to_dir = _option_value(args, _TARGET_DIR_OPTIONS)
    trimmed = _without_option_value(args, _TARGET_DIR_OPTIONS)
    operands = _operands(trimmed)
    if to_dir is not None:
        for source in operands:
            _add(found, os.path.join(to_dir, os.path.basename(source)), where)
        return
    if not operands:
        return
    destination = operands[-1]
    sources = operands[:-1]
    if program == "mv":
        for source in sources:
            _add(found, source, where)
    if sources and _is_dir(destination, where):
        for source in sources:
            _add(found, os.path.join(destination, os.path.basename(source)), where)
    else:
        _add(found, destination, where)


def _without_bundled_value(args: list[str], tails: str) -> list[str]:
    """``args`` without a short-option bundle that ends in one of ``tails``
    (perl's ``-pe``, ``-ine``): that trailing letter takes the next word as
    its value, as a separate ``-e <script>`` would."""
    out: list[str] = []
    index = 0
    while index < len(args):
        arg = args[index]
        if (
            arg.startswith("-")
            and not arg.startswith("--")
            and len(arg) >= 3
            and arg[-1] in tails
            and arg[1:].isalpha()
            and index + 1 < len(args)
        ):
            index += 2
            continue
        out.append(arg)
        index += 1
    return out


def _without_option_value(args: list[str], names: tuple[str, ...]) -> list[str]:
    """``args`` without every ``-t <dir>``-style pair (attached forms survive:
    they are one word)."""
    if not names:
        return args
    out: list[str] = []
    index = 0
    while index < len(args):
        if any(args[index] == name for name in names) and index + 1 < len(args):
            index += 2
            continue
        out.append(args[index])
        index += 1
    return out


def _has_script_option(args: list[str]) -> bool:
    """Whether sed's program came as an option (``-e``, ``-f``, ``-ne``,
    ``--expression=``): the operands are then all files."""
    for arg in args:
        if arg in ("-e", "-f", "--expression", "--file"):
            return True
        if arg.startswith("--expression=") or arg.startswith("--file="):
            return True
        if arg.startswith("-") and not arg.startswith("--") and len(arg) > 2:
            if "e" in arg or "f" in arg:
                return True
    return False


def _perl_scripted(args) -> bool:
    """Whether perl's program came as an option (``-e``, ``-E``, the bundled
    ``-ne``/``-pE``, ``--execute=``): the operands are then all files.
    Without it the first operand is the program file (S4R-rules-23).
    Perl's ``-f`` takes no value and scripts nothing (S4R-rules-C2-3).
    ``M``, ``F`` and ``I`` take the rest of their word as a value, so
    ``-MData::Dumper`` is a module name, never a script option."""
    for arg in args:
        if arg in ("-e", "-E"):
            return True
        if arg.startswith("--execute") or arg.startswith("--eval"):
            return True
        if not arg.startswith("-") or arg.startswith("--") or len(arg) < 2:
            continue
        for char in arg[1:]:
            if char in _PERL_VALUE_LEADERS:
                break  # the rest of the word is that option's value
            if char in "eE":
                return True
    return False


def _is_dir(destination: str, where: str) -> bool:
    try:
        text = destination
        if not text.startswith("/"):
            if not where:
                return False
            text = os.path.join(where, text)
        return os.path.isdir(text)
    except OSError:
        return False


def _sed_in_place(arg: str) -> bool:
    """``-i``, ``-ibak`` (an attached suffix), ``--in-place``, ``--in-place=bak``."""
    if arg == "-i" or arg == "--in-place" or arg.startswith("--in-place="):
        return True
    return arg.startswith("-i") and len(arg) > 2


def _perl_in_place(arg: str) -> bool:
    """``-i``, ``-i.bak``, or ``i`` bundled before a value-less option
    (``-pi``, ``-lpei``). The letters ``M``, ``F`` and ``I`` take the rest of
    the word as their value, so ``-Mail`` is a module named ``ail`` and never
    in place (S4R-rules-23)."""
    if arg == "-i" or arg.startswith("-i"):
        return True
    if not arg.startswith("-") or arg.startswith("--") or len(arg) < 3:
        return False
    for char in arg[1:]:
        if char == "i":
            return True
        if char in _PERL_VALUE_LEADERS:
            return False  # the rest of the word is that option's value
        if not char.isalpha():
            return False  # a separator ends the bundled letters
    return False
