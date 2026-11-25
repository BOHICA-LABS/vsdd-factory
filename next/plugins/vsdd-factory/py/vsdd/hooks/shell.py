"""The Bash command splitter (plan 3.6; [[topic:rule-model]] ``where``).

``parse(text, cwd) -> Command`` splits one POSIX sh command into segments at
its joiners (``;``, ``&&``, ``||``, ``|``, ``&``, newline), and each segment
into assignments, the program, the arguments and the redirects. ``where`` is
the directory a rule tests — the segment's own ``git -C``/``vsdd -C``, the
``cd`` before it, or ``cwd`` — and ``""`` when unknowable: a target spelled
with an expansion or a glob is not guessed. ``opaque`` marks the constructs
the parser does not enter: a substitution, a subshell, a here-document,
``eval``, ``sh -c`` and ``xargs``; a rule sees such a segment whole. An
arithmetic span (``$((...))``) is not one of them — it evaluates, it runs
nothing — while it is plain arithmetic: it closes with ``))``, its inner
parentheses balance, and it holds only arithmetic characters. ``$((cmd) )``
and ``$((a)&&(b))`` are command substitutions (sh and bash end the span at
the ``)`` that closes its second ``(`` and reparse the rest —
``$((echo PWNED)|(cat))`` runs code), and a span nesting a command
substitution or a backtick runs code: all are opaque. A ``$[`` is bash and
zsh arithmetic whose extent is not the word loop's, and a ``${...}`` span
whose inner text is not plainly a name, an operator or ``$name`` — a quote
inside can hide the span's real end (``${y:-'}'$(id)}``) — counts a
substitution too, inside double quotes as well, where ``$[`` counts one and
a ``$'`` (ANSI-C quoting in bash and zsh, POSIX text in dash) always does.


The dialect is what ``sh`` accepts, which is what the Bash tool runs the
commands through; a bash-only keyword is read as an ordinary program name,
which is enough for what a rule may judge about it. ``parse`` never raises: an
internal fault degrades to one opaque segment carrying the whole text, never
to a wrong fact.
"""

from __future__ import annotations

import os
import re
from dataclasses import dataclass

__all__ = ["Redirect", "Segment", "Command", "parse", "GIT_GLOBAL_VALUE_OPTIONS",
           "GIT_FREE_TEXT_OPTIONS", "free_text_value_words", "HEREDOC_SHELL_READERS"]

# No "\r": a carriage return is an ordinary word character in sh, bash, dash
# and zsh, so it cannot end a word or start a comment
# (step3-hook-runner-fix-3-C1-2).
_WORD_END = set(" \t\n;&|()<>")

_REDIRECT_OPS = (
    "<<-",
    "<<<",
    "<<",
    ">>",
    ">&",
    "<&",
    "<>",
    "&>>",
    "&>",
    ">|",
    ">",
    "<",
)

_ASSIGNMENT_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*=")

#: Programs whose interior the parser does not enter.
_OPAQUE_PROGRAMS = ("eval", "xargs")

#: Leading programs that change nothing about what the wrapped call is: the
#: real program follows them, so the segment's program is the wrapped one
#: (``env FOO=1 git commit`` is a git call; S4R-rules-7). ``env``'s operand
#: assignments still read as the assignments they are, because the scan
#: stays ``at_program`` past a wrapper. A wrapper that runs its operand as a
#: command is here with its option grammar, so the wrapped git call is read
#: and judged at the place git would act; the common ones sit in the table
#: and an unknown one stays for merge-guard's opaque ruling to cover
#: (S4R-rules-C2-2).
_WRAPPER_PROGRAMS = frozenset(
    {"command", "env", "exec", "nohup", "time", "nice",
     "timeout", "stdbuf", "setsid", "ionice", "sudo"}
)

#: The options of the wrappers that stand alone (S4R-rules-7). Another
#: option-shaped word after a wrapper cannot be parsed here, and the segment
#: counts opaque: the wrapped program is not guessed.
_WRAPPER_FLAGS = {
    "env": ("-i", "--ignore-environment"),
    "nice": (),
    "command": ("-p",),
    "time": ("-p",),
    "timeout": ("--preserve-status", "--foreground", "-v", "--verbose"),
    "stdbuf": (),
    "setsid": ("-w", "--wait", "-c", "--ctty", "--fork"),
    "ionice": (),
    "sudo": ("-b", "-E", "-e", "-H", "-h", "-i", "-k", "-K", "-l", "-n", "-s", "-v",
             "--preserve-env", "--login", "--non-interactive", "--reset-timestamp"),
}

#: The wrapper options that take a separate value word (``env -u NAME``,
#: ``nice -n 5``, ``timeout -s TERM``, ``stdbuf -o 0``, ``sudo -u root``), and
#: the long spellings that carry theirs attached (``--unset=NAME``). One row
#: per wrapper: a shared table has sudo's ``-R`` (--chroot) and ``-D``
#: (--chdir) paired wrongly, and env's ``-C``/``--chdir`` lose their value
#: (S4R-rules-C2-2).
_WRAPPER_VALUE_FLAGS = {
    "env": ("-u", "--unset", "-S", "--split-string", "-C", "--chdir"),
    "nice": ("-n", "--adjustment"),
    "timeout": ("-s", "--signal", "-k", "--kill-after"),
    "stdbuf": ("-i", "-o", "-e"),
    "ionice": ("-c", "--class", "-n", "--priority"),
    "sudo": ("-u", "--user", "-g", "--group", "-p", "--prompt", "-C", "--close-from",
             "-R", "--chroot", "-D", "--chdir", "-T", "--command-timeout"),
}
_WRAPPER_LONG_VALUE_PREFIXES = {
    "env": ("--unset=", "--split-string=", "--chdir="),
    "nice": ("--adjustment=",),
    "timeout": ("--signal=", "--kill-after="),
    "stdbuf": (),
    "ionice": ("--class=", "--priority="),
    "sudo": ("--user=", "--group=", "--prompt=", "--close-from=", "--chdir=",
             "--chroot=", "--command-timeout=", "--preserve-env="),
}

#: The wrapper options that change the directory the wrapped program runs in,
#: and the one that changes the root it sees. A ``--chdir`` value resolves like
#: a ``cd`` for the wrapped call; a ``--chroot`` moves the whole tree, so no
#: directory outside can be related to it (S4R-rules-C2-2).
_WRAPPER_CHDIR_OPTIONS = {"env": ("-C", "--chdir"), "sudo": ("-D", "--chdir")}
_WRAPPER_CHROOT_OPTIONS = ("sudo",)

#: The interpreters a here-document's reader names when its body is the
#: commands themselves: the body stays part of such a segment's text, where
#: the body of ``cat``, ``tee`` or ``git commit -F -`` is data and stays out
#: (step4-rules-fix-3-C1-2).
HEREDOC_SHELL_READERS = frozenset({"sh", "bash", "dash", "zsh", "ksh"})

#: The wrappers that take a plain operand of their own before the wrapped
#: program: ``timeout 10 git commit`` spends ``10`` on the duration
#: (S4R-rules-C2-2).
_WRAPPER_LEADING_OPERANDS = {"timeout": 1}


@dataclass(frozen=True)
class Redirect:
    """One redirect: the operator and its target as typed."""

    op: str
    target: str


@dataclass(frozen=True)
class Segment:
    """One simple command of the parse; ``joiner`` is the operator before it
    (``""`` for the first, a newline for a newline-separated one)."""

    text: str
    joiner: str
    assignments: tuple[tuple[str, str], ...]
    program: str
    args: tuple[str, ...]
    redirects: tuple[Redirect, ...]
    where: str
    opaque: bool


@dataclass(frozen=True)
class Command:
    """The whole parse of one command text."""

    text: str
    segments: tuple[Segment, ...]
    opaque: bool


class _Word:
    """A lexical word: its value with the quotes removed, whether that value
    is a guessable literal or carries an expansion or a glob, and whether a
    command substitution (``$(..)``, a backtick span) sits inside it. An
    arithmetic span counts only when it is a plain one: ``$((1+2))``
    evaluates, it runs nothing, while ``$((cmd) )`` is a command
    substitution and a span that nests one runs code."""

    __slots__ = ("text", "expanded", "subst")

    def __init__(self, text: str, expanded: bool = False, subst: bool = False):
        self.text = text
        self.expanded = expanded
        self.subst = subst


# --- lexer -------------------------------------------------------------------


def _lex(text: str) -> list[tuple[str, object]]:
    """Tokenise into ``("word", _Word)`` and ``("op", str)``; comments drop."""
    tokens: list[tuple[str, object]] = []
    index, size = 0, len(text)
    while index < size:
        char = text[index]
        if char in " \t":
            index += 1
            continue
        if char == "\n":
            tokens.append(("op", "\n"))
            index += 1
            continue
        if char == "#":
            end = text.find("\n", index)
            index = size if end < 0 else end
            continue
        if char in ";&|":
            pair = text[index : index + 2]
            if pair in ("&&", "||"):
                tokens.append(("op", pair))
                index += 2
                continue
            if char == "&":
                # &> and &>> are redirects, not a background joiner followed
                # by a redirection of the next segment.
                op, width = _redirect_op(text, index)
                if op is not None:
                    tokens.append(("op", op))
                    index += width
                    continue
            tokens.append(("op", char))
            index += 1
            continue
        if char in "()":
            tokens.append(("op", char))
            index += 1
            continue
        if char in "<>&" or (char.isdigit() and _fd_hugs_redirect(text, index)):
            op, width = _redirect_op(text, index)
            if op is not None:
                tokens.append(("op", op))
                index += width
                continue
            if char in "<>&":
                tokens.append(("op", char))
                index += 1
                continue
        word, index = _read_word(text, index)
        tokens.append(("word", word))
    return tokens


def _redirect_op(text: str, index: int) -> tuple[str | None, int]:
    """The redirect operator at ``index``: a long form, or an fd number — any
    run of digits, as POSIX io_number allows (``2>``, ``10>>``) — or ``&``
    hugging a form (``>&``, ``&>``). Longest wins."""
    for form in _REDIRECT_OPS:
        if text.startswith(form, index):
            return form, len(form)
    # An fd number hugging the operator: 2>, 10>>
    if text[index].isdigit():
        probe = index
        while probe < len(text) and text[probe].isdigit():
            probe += 1
        if probe < len(text):
            for form in _REDIRECT_OPS:
                if text.startswith(form, probe):
                    return f"{text[index:probe]}{form}", probe - index + len(form)
    return None, 0


def _fd_hugs_redirect(text: str, index: int) -> bool:
    """Whether the run of digits at ``index`` is hugged by a redirect
    operator (``2>log``, ``10>/dev/null``): the number is then that
    operator's fd, not a word (step3-hook-runner-fix-3-C1-3)."""
    probe, size = index, len(text)
    while probe < size and text[probe].isdigit():
        probe += 1
    return probe < size and text[probe] in "<>"


def _read_word(text: str, index: int) -> tuple[_Word, int]:
    """One word from ``index``: quotes, escapes and substitution spans read as
    their as-typed content; the word ends at a boundary character."""
    size = len(text)
    parts: list[str] = []
    expanded = False
    subst = False
    while index < size:
        char = text[index]
        if char in _WORD_END:
            break
        if char == "'":
            end = text.find("'", index + 1)
            if end < 0:
                parts.append(text[index + 1 :])
                index = size
                break
            parts.append(text[index + 1 : end])
            index = end + 1
            continue
        if char == '"':
            index += 1
            while index < size and text[index] != '"':
                if text[index] == "\\" and index + 1 < size and text[index + 1] in '"\\$`':
                    parts.append(text[index + 1])
                    index += 2
                    continue
                if text[index] in "$`":
                    expanded = True
                    if text[index] == "`":
                        subst = True
                    elif _opens_arithmetic(text, index):
                        # OR-ed in, never assigned: an arithmetic span later
                        # in the word must not unsee an earlier substitution
                        # (S3R-hook-runner-C2-1).
                        if not _arithmetic_harmless(text, index):
                            subst = True
                    elif text.startswith("$(", index):
                        subst = True
                    elif text.startswith("${", index):
                        # The plain-text rule holds inside quotes too: a quote
                        # nested in the span hides its real end from the first
                        # '}' while the shell reads past it, and an escaped
                        # '\$' plants code a plain-looking reader then runs
                        # (S3R-hook-runner-C2-1). Only a plain span stays a
                        # literal; an unclosed one is not read at all.
                        close = text.find("}", index)
                        inner = text[index + 2 :] if close < 0 else text[index + 2 : close]
                        if close < 0 or not _parameter_plain(inner):
                            subst = True
                    elif text.startswith("$[", index):
                        # bash and zsh arithmetic: its extent is not the word
                        # loop's, and the escaped '\$' inside hides nothing
                        # (step3-hook-runner-fix-3-C1-1).
                        subst = True
                parts.append(text[index])
                index += 1
            index += 1  # the closing quote
            continue
        if char == "\\":
            if index + 1 < size:
                following = text[index + 1]
                if following == "\n":  # line continuation joins the words
                    index += 2
                    continue
                parts.append(following)
                expanded = expanded or following in "$`"
                index += 2
                continue
            index += 1
            continue
        if char == "$":
            expanded = True
            if text.startswith("$(", index):
                if _opens_arithmetic(text, index):
                    # $((...)) evaluates; it runs nothing — but only when it
                    # is a plain arithmetic span: $((cmd) ) runs a command
                    # substitution in sh and bash, and a nested $( or backtick
                    # runs code. OR-ed in, never assigned: an arithmetic span
                    # later in the word must not unsee an earlier
                    # substitution (S3R-hook-runner-C2-1).
                    if not _arithmetic_harmless(text, index):
                        subst = True
                else:
                    subst = True
                end = _span_substitution(text, index)
                parts.append(text[index:end])
                index = end
                continue
            if text.startswith("${", index):
                close = text.find("}", index)
                if close < 0:
                    inner, end = text[index + 2 :], size
                else:
                    inner, end = text[index + 2 : close], close + 1
                if not _parameter_plain(inner):
                    # A quote or a brace quoted inside the span can hide its
                    # real end from the first '}' while the shell reads past
                    # it (S3R-hook-runner-C2-1): only a plain span stays a
                    # literal.
                    subst = True
                parts.append(text[index:end])
                index = end
                continue
            if text.startswith("$[", index):
                # bash and zsh arithmetic ($[$(id)]): its extent is not the
                # word loop's, so any $[ counts a substitution
                # (step3-hook-runner-fix-3-C1-1).
                subst = True
            if text.startswith("$'", index):
                # bash and zsh read $'...' as ANSI-C quoting, where \' does
                # not end the string; dash reads the POSIX way. The shells
                # disagree on where it ends, so it is never read as plain
                # text (step3-hook-runner-fix-3-C2-1).
                subst = True
            parts.append(char)
            index += 1
            continue
        if char == "`":
            expanded = True
            subst = True
            end = text.find("`", index + 1)
            end = size if end < 0 else end + 1
            parts.append(text[index:end])
            index = end
            continue
        if char == "*":
            expanded = True
            parts.append(char)
            index += 1
            continue
        if char == "?":
            expanded = True
            parts.append(char)
            index += 1
            continue
        if char == "[":
            # A '[' is no construct of its own: a shell ends the word at a
            # metacharacter whatever brackets are open, so the loop reads on
            # and only the glob sense of the bracket remains — blanks,
            # joiners, '$(' and backticks after it are read as anywhere else
            # (step3-hook-runner-fix-3-C1-1).
            expanded = True
            parts.append(char)
            index += 1
            continue
        parts.append(char)
        index += 1
    return _Word("".join(parts), expanded, subst), index


def _opens_arithmetic(text: str, index: int) -> bool:
    """Whether the ``$((`` of an arithmetic span opens at ``index``."""
    return text.startswith("$((", index)


#: What a plain arithmetic span may hold: digits, names, ``$name``, operators,
#: blanks and grouping. A quote, a semicolon or a backtick does not parse as
#: arithmetic, and ``$(`` hides in a per-character check, so both spell
#: trouble out loud below.
_ARITHMETIC_CHARS = frozenset(
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
    "_+-*/%()<>=!&|^~:,.$ \t"
)


def _arithmetic_harmless(text: str, start: int) -> bool:
    """Whether the ``$((`` opening at ``start`` begins a span that is plain
    arithmetic: it closes with ``))``, its inner parentheses balance, and it
    holds only arithmetic characters, no ``$(`` and no backtick. Such a span
    evaluates, it runs nothing. Any other ``$((`` — ``$((cmd) )``, which sh
    and bash run as a command substitution, ``$((a)&&(b))``, whose span ends
    at the ``)`` that closes its second ``(``, an unclosed one, one nesting
    ``$(...)`` — can run code, so the caller counts it a substitution."""
    end = _span_substitution(text, start)
    if end - start < 5 or not text.startswith("))", end - 2):
        return False
    inner = text[start + 3 : end - 2]
    return (
        "$(" not in inner
        and "`" not in inner
        and _parentheses_balance(inner)
        and all(char in _ARITHMETIC_CHARS for char in inner)
    )


def _parentheses_balance(span: str) -> bool:
    """Whether every ``(`` in ``span`` closes with a ``)`` and none closes
    unopened: sh and bash end an arithmetic span at the ``)`` that closes its
    second ``(`` and reparse the rest as a command substitution, so however
    arithmetic-looking, ``$((a)&&(b))`` and ``$((echo PWNED)|(cat))`` run
    code (S3R-hook-runner-C2-1)."""
    depth = 0
    for char in span:
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth < 0:
                return False
    return depth == 0


#: What the inner text of a ``${...}`` span may hold for the word loop's
#: reading of it to be the shell's: a name, the usual operators and
#: separators, blanks, ``$name`` and plain arithmetic. Anything else — a
#: quote, a backslash, a nested ``${`` or a parenthesis — can
#: hide the span's real end from the first ``}``, so it counts a
#: substitution (S3R-hook-runner-C2-1). ``@`` and ``!`` are out: bash 4.4+'s
#: ``${name@P}`` expands the value as a prompt string, which runs any
#: ``$(...)`` in it, and ``${!x}`` is indirect expansion through array
#: subscripts (step3-hook-runner-fix-3-C2-2). ``${@}`` then reads opaque,
#: which is safe.
_PARAM_PLAIN_CHARS = frozenset(
    "0123456789abcdefghijklmnopqrstuvwxyz"
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    "_-.,/:=+%#~ \t"
)


def _parameter_plain(span: str) -> bool:
    """Whether the inner text of a ``${...}`` span is plain: a name, an
    operator, ``$name`` or a plain arithmetic span, and nothing else."""
    index, size = 0, len(span)
    while index < size:
        char = span[index]
        if char == "$":
            if span.startswith("$(", index):
                if _opens_arithmetic(span, index) and _arithmetic_harmless(span, index):
                    index = _span_substitution(span, index)
                    continue
                return False
            if span.startswith("${", index):
                return False  # a nested ${...} span is not re-read
            following = span[index + 1] if index + 1 < size else ""
            if not (following.isalnum() or following == "_"):
                return False
            index += 2
            while index < size and (span[index].isalnum() or span[index] == "_"):
                index += 1
            continue
        if char in _PARAM_PLAIN_CHARS:
            index += 1
            continue
        return False
    return True


def _span_substitution(text: str, start: int) -> int:
    """The index past the ``$(...)`` that opens at ``start``."""
    depth = 0
    index, size = start, len(text)
    while index < size:
        char = text[index]
        if char == "'":
            close = text.find("'", index + 1)
            index = size if close < 0 else close + 1
            continue
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                return index + 1
        index += 1
    return size


# --- here-documents -----------------------------------------------------------


def _cut_heredocs(text: str) -> tuple[str, list[tuple[str, str]]]:
    """Replace here-document bodies with nothing, so their lines are never
    read as commands; return the cleaned text and the ``(delimiter, body)``
    pairs in order of appearance. The caller hangs a body back on the segment
    whose reader runs it (a shell interpreter; step4-rules-fix-3-C1-2) — the
    body of ``cat``, ``tee`` or ``git commit -F -`` is data and stays out."""
    if "<<" not in text:
        return text, []
    pieces: list[str] = []
    bodies: list[tuple[str, str]] = []
    index, size = 0, len(text)
    while index < size:
        char = text[index]
        if char == "\\":
            pieces.append(text[index : index + 2])
            index += 2
            continue
        if char in "'\"":
            close = _matching_quote(text, index, char)
            pieces.append(text[index:close])
            index = close
            continue
        if char == "<" and text.startswith("<<", index) and _heredoc_starts_op(text, index):
            delimiter, after = _heredoc_delimiter(text, index)
            if delimiter is not None:
                end = _heredoc_end(text, after, delimiter)
                pieces.append(text[index:after])
                # Everything to the terminator's own newline is body; the
                # command text keeps its line break.
                body_start = text.find("\n", after)
                body = (text[body_start + 1 : end] if 0 <= body_start < end
                        else "")
                bodies.append((delimiter, body.rstrip("\n")))
                pieces.append("\n" if end < size and text[end] != "\n" else "")
                index = end
                continue
        pieces.append(char)
        index += 1
    return "".join(pieces), bodies


def _matching_quote(text: str, index: int, quote: str) -> int:
    """Past the closing quote of the one that opens at ``index``."""
    index += 1
    size = len(text)
    while index < size:
        char = text[index]
        if char == "\\":
            index += 2
            continue
        if char == quote:
            return index + 1
        index += 1
    return size


def _heredoc_starts_op(text: str, index: int) -> bool:
    """``<<`` introduces a here-document when nothing rides before it: a
    blank, an operator or the line start."""
    if index == 0:
        return True
    before = text[index - 1]
    return before in " \t\n;&|(<>" or before.isdigit() or before == "&"


def _heredoc_delimiter(text: str, index: int) -> tuple[str | None, int]:
    """The delimiter word after the ``<<`` at ``index``; quotes stripped."""
    size = len(text)
    index += 2
    if index < size and text[index] == "-":
        index += 1
    while index < size and text[index] in " \t":
        index += 1
    if index >= size or text[index] == "\n":
        return None, index
    if text[index] in "'\"":
        quote = text[index]
        close = text.find(quote, index + 1)
        if close < 0:
            return None, index
        return text[index + 1 : close], close + 1
    start = index
    while index < size and text[index] not in " \t\n;&|<>()":
        index += 1
    return (text[start:index] or None), index


def _heredoc_end(text: str, after: int, delimiter: str) -> int:
    """The start of the line that closes the body: the first line equal to the
    delimiter (leading tabs allowed, for ``<<-``), searched after the opener."""
    newline = text.find("\n", after)
    while newline != -1 and newline + 1 < len(text):
        line_end = text.find("\n", newline + 1)
        line_end = len(text) if line_end < 0 else line_end
        if text[newline + 1 : line_end].strip("\t") == delimiter:
            return line_end + 1 if line_end < len(text) else len(text)
        newline = line_end
    return len(text)


# --- assembly ------------------------------------------------------------------


def parse(text: str, cwd: str) -> Command:
    """Split one command; never raises. An internal fault degrades to one
    opaque segment carrying the whole text."""
    try:
        return _parse(text, cwd)
    except Exception:
        inert = Segment(
            text=text, joiner="", assignments=(), program="", args=(),
            redirects=(), where=os.path.abspath(cwd) if cwd else "", opaque=True,
        )
        return Command(text=text, segments=(inert,), opaque=True)


def _parse(text: str, cwd: str) -> Command:
    cleaned, bodies = _cut_heredocs(text)
    tokens = _lex(cleaned)
    pending = list(bodies)

    # Group the tokens into simple commands at the joiners. A top-level
    # subshell swallows its tokens: its `cd` moves nothing outside it.
    groups: list[tuple[str, list[object], bool]] = []  # (joiner, tokens, subshell)
    current: list[object] = []
    joiner = ""
    depth = 0
    for kind, value in tokens:
        if kind == "op" and value == "(":
            depth += 1
            if depth == 1:
                if current:
                    groups.append((joiner, current, False))
                    current = []
                groups.append((joiner, [], True))
                joiner = ""
                continue
        if kind == "op" and value == ")":
            depth = max(0, depth - 1)
            if depth == 0:
                # The subshell keeps its own tokens as its segment's text, so
                # a rule reads what the subshell itself holds and never the
                # whole command's (S4R-rules-3).
                if groups and groups[-1][2] and not groups[-1][1]:
                    groups[-1] = (groups[-1][0], current, True)
                current = []
                joiner = ""
                continue
        if kind == "op" and value in (";", "&&", "||", "|", "&", "\n") and depth == 0:
            if current:
                groups.append((joiner, current, False))
                current = []
            joiner = "\n" if value == "\n" else value
            continue
        current.append((kind, value))
    if current:
        groups.append((joiner, current, False))

    # Build the segments and thread `where` through them: `cd` moves the
    # later segments; `git -C` and `vsdd -C` name their own segment's spot. A
    # CDPATH set in an earlier segment sends a relative `cd` who knows where.
    where_now = os.path.abspath(cwd) if cwd else ""
    cdpath = False
    segments: list[Segment] = []
    for joiner, group, subshell in groups:
        if subshell:
            # The segment carries the text the subshell stands for — its own
            # words and operators — so a rule judges what it holds and never
            # re-scans the whole command (S4B-rules-2). An empty subshell
            # stands for nothing: its text is "". A subshell whose text holds
            # a cd runs somewhere its own text does not resolve: its `where`
            # is "" rather than a wrong fact (S4B-rules-2).
            inner = " ".join(
                value.text if kind == "word" else value for kind, value in group
            )
            taken = _consume_bodies(group, pending)
            # A here-document a subshell's interior reads is shell text the
            # subshell runs: it rides the segment's text like the rest.
            held = " ".join(body for _name, body in taken if body)
            if held:
                inner = f"{inner} {held}" if inner else held
            moves = any(
                kind == "word" and value.text in ("cd", "pushd", "popd")
                for kind, value in group
            )
            segments.append(
                Segment(inner, joiner, (), "", (), (),
                        "" if moves else where_now, True)
            )
            continue
        segment, hop = _build_segment(joiner, group, where_now, pending, cdpath)
        segments.append(segment)
        if hop is not None:
            where_now = hop
        if _sets_cdpath(segment):
            cdpath = True
    opaque = any(segment.opaque for segment in segments)
    return Command(text=text, segments=tuple(segments), opaque=opaque)


def _consume_bodies(group: list[object], pending: list[tuple[str, str]]) -> list[tuple[str, str]]:
    """Take the here-document bodies this segment's ``<<`` openers read, in
    order, so the later segments keep their own."""
    taken: list[tuple[str, str]] = []
    for index, (kind, value) in enumerate(group):
        if kind == "op" and value in ("<<", "<<-"):
            following = group[index + 1] if index + 1 < len(group) else None
            delimiter = (
                following[1].text if following and following[0] == "word" else ""
            )
            taken.append(_take_body(pending, delimiter))
    return taken


def _take_body(pending: list[tuple[str, str]], delimiter: str) -> tuple[str, str]:
    """The recorded body whose opener read ``delimiter`` — the first still
    pending whose name matches, else the first at all."""
    for position, (name, body) in enumerate(pending):
        if name == delimiter:
            return pending.pop(position)
    return pending.pop(0) if pending else ("", "")


#: The POSIX reserved words a leading word may carry before the program; and
#: the negation operator, which reads as a word.
_RESERVED_WORDS = frozenset(
    {"if", "then", "elif", "else", "fi", "while", "until", "for", "do", "done",
     "case", "esac", "!", "{"}
)


def _build_segment(joiner: str, tokens: list[object], where: str,
                   pending: list[tuple[str, str]], cdpath: bool) -> tuple[Segment, str | None]:
    """Assignments, program, arguments and redirects of one simple command.
    Leading reserved words (``then``, ``!``) are stripped, so a compound
    command's parts are judged by their real program; a ``for`` or ``case``
    header is not a command at all and is marked opaque; a substitution in any
    word — the program, an assignment value, a redirect target — makes the
    segment opaque. Here-document opacity is the segment's own: the segment
    holding the ``<<`` reads its body from ``pending``, and a shell reader
    carries the body in its text (step4-rules-fix-3-C1-2). Returns the segment
    and the ``where`` a ``cd`` in it imposes on the later segments (``None``:
    the segment moves nothing)."""
    text_parts: list[str] = []
    assignments: list[tuple[str, str]] = []
    program = ""
    args: list[str] = []
    arg_words: list[_Word] = []
    redirects: list[Redirect] = []
    bodies: list[tuple[str, str]] = []
    opaque = False
    at_program = True
    header = False  # a for/case header: not a command, judged by no program
    body_group = False  # a { ... } group or function body: runs where it is called

    index = 0
    count = len(tokens)
    while index < count:
        kind, value = tokens[index]
        if kind == "op" and _is_redirect(value):
            target = ""
            delimiter = ""
            if index + 1 < count and tokens[index + 1][0] == "word":
                target_word: _Word = tokens[index + 1][1]  # type: ignore[index]
                if target_word.subst:
                    opaque = True  # the redirection runs a substitution
                target = target_word.text
                delimiter = target_word.text if value in ("<<", "<<-") else ""
                index += 2
            else:
                index += 1
            if value in ("<<", "<<-"):
                # The body is unseen; the parser stays out. The segment takes
                # the body its opener read, for its text and its rules.
                opaque = True
                bodies.append(_take_body(pending, delimiter))
            redirects.append(Redirect(value, target))
            continue
        if kind == "op":
            opaque = True  # parentheses inside the segment: not entered
            index += 1
            continue
        word = value  # type: ignore[misc]
        text_parts.append(word.text)  # type: ignore[attr-defined]
        if word.subst:
            opaque = True  # a substitution in any word is not entered
        if at_program and _ASSIGNMENT_RE.match(word.text):  # type: ignore[attr-defined]
            name, _, assigned = word.text.partition("=")  # type: ignore[attr-defined]
            assignments.append((name, assigned))
            index += 1
            continue
        if at_program and word.text in _RESERVED_WORDS:
            if word.text in ("for", "case"):
                header = True  # the header of a compound command, no command
            if word.text == "{":
                # A group or function body: its commands run wherever the
                # group is called, a place the definition's directory does
                # not give (step4-rules-fix-3-C1-5).
                body_group = True
            index += 1  # stripped: the program is the first non-reserved word
            continue
        if at_program:
            wrapper = _wrapper_name(word.text)  # type: ignore[attr-defined]
            if wrapper is not None:
                skip, hidden, pieces, chdir = _wrapper_options(wrapper, tokens, index + 1)
                text_parts.extend(pieces)
                if skip is None or hidden:
                    # A wrapper option that cannot be parsed, or one the
                    # parser does not enter (env's split string): the wrapped
                    # program is not guessed, and the segment counts opaque
                    # so a rule fails closed (S4R-rules-7).
                    opaque = True
                if skip is not None:
                    if chdir is not None:
                        # The wrapper moved the wrapped program's directory:
                        # --chdir resolves like a cd, --chroot like a leap
                        # into another tree (S4R-rules-C2-2).
                        where = _chdir_target(chdir, where) if chdir[0] else ""
                    index = skip
                    continue
            program = word.text  # type: ignore[attr-defined]
            at_program = False
            index += 1
            continue
        args.append(word.text)  # type: ignore[attr-defined]
        arg_words.append(word)
        index += 1

    if program in ("git", "vsdd"):
        hop = _minus_c(_global_args(arg_words), where)
        if hop is not None:
            where = hop
    opaque = opaque or header or _own_opacity(program, args)
    if opaque:
        held = " ".join(body for _name, body in bodies if body)
        reader = os.path.basename(program) in HEREDOC_SHELL_READERS
        if held and reader:
            # The here-document feeds a shell interpreter: its body is the
            # commands the segment runs, so the text carries it
            # (step4-rules-fix-3-C1-2).
            text_parts.append(held)
    # A cd moves the later segments. A bare cd goes home, and a cd whose
    # target is an expansion, a tilde or `-` moves somewhere unknowable: both
    # leave "" rather than a wrong fact. The directory-stack builtins move
    # where their stack decides: also "".
    move: str | None = None
    if program == "cd":
        move = _cd_target(arg_words, where, cdpath)
    elif program in ("pushd", "popd"):
        move = ""
    if body_group:
        opaque = True
        where = ""  # the body runs at its call site, not here
    return (
        Segment(
            text=" ".join(text_parts),
            joiner=joiner,
            assignments=tuple(assignments),
            program=program,
            args=tuple(args),
            redirects=tuple(redirects),
            where=where,
            opaque=opaque,
        ),
        move,
    )


def _sets_cdpath(segment: Segment) -> bool:
    """Whether the segment sets CDPATH, so a later relative ``cd`` may reach
    any of its directories (step4-rules-fix-3-C1-5)."""
    if any(name == "CDPATH" for name, _value in segment.assignments):
        return True
    if os.path.basename(segment.program) in ("export", "set"):
        return any(arg.startswith("CDPATH=") for arg in segment.args)
    return False


def _chdir_target(pair: tuple[str, str], where: str) -> str:
    """The directory a wrapper's ``--chdir <dir>`` names, resolved; ``""``
    when it is not guessable."""
    _flag, value = pair
    return _resolve(value, where)


def _wrapper_name(text: str) -> str | None:
    """The wrapper a leading word names — by its own basename, wherever it is
    mounted (``/usr/bin/env`` is env; S4R-rules-7) — or ``None``."""
    name = os.path.basename(text) if text else ""
    return name if name in _WRAPPER_PROGRAMS else None


def _wrapper_options(wrapper: str, tokens: list[object], index: int):
    """The passage past ``wrapper``'s own options, so the wrapped program
    follows (``env -i git commit`` is a git call; S4R-rules-7). Returns
    ``(skip, hidden, pieces, chdir)``: ``skip`` is the index of the first
    token past the wrapper's options and operands (``None`` when an option
    there cannot be parsed — the caller then marks the segment opaque instead
    of guessing the wrapped program); ``hidden`` says the parser does not
    enter what follows (``env -S``'s value is the command, and it is entered
    only as text; S4R-rules-7); ``pieces`` are the quote-removed texts
    consumed here, so the segment's text still says what was eaten; and
    ``chdir`` is the ``(flag, value)`` of the wrapper's directory option,
    when it named one (S4R-rules-C2-2)."""
    count = len(tokens)
    pieces: list[str] = []
    chdir: tuple[str, str] | None = None
    while index < count:
        kind, value = tokens[index]
        if kind != "word":
            return None, False, pieces, chdir  # a redirect or a parenthesis amid the options
        word = value.text  # type: ignore[attr-defined]
        if word == "--":  # end of the wrapper's options
            index += 1
            break
        if not word.startswith("-") or len(word) == 1:
            break  # an assignment, a leading operand, or the program itself
        if word in ("-S", "--split-string"):
            # The split string IS the command env runs; the parser does not
            # enter its word splitting, and no program word follows it — the
            # scan ends here and the segment counts opaque, its text carrying
            # the split string verbatim (S4R-rules-7).
            if index + 1 < count and tokens[index + 1][0] == "word":
                pieces.extend([word, tokens[index + 1][1].text])  # type: ignore[attr-defined]
                return count, True, pieces, chdir
            return None, False, pieces, chdir  # the value is missing
        attached = _attached_split_string(word)
        if attached is not None:
            # env -S"cmd" spells option and value as one shell word; the
            # pieces restate them separately, so the text holds the command's
            # words (S4R-rules-7).
            pieces.extend(["-S", attached])
            return count, True, pieces, chdir
        if any(word.startswith(prefix)
               for prefix in _WRAPPER_LONG_VALUE_PREFIXES.get(wrapper, ())):
            pieces.append(word)
            if word.startswith("--chdir="):
                chdir = ("--chdir", word[len("--chdir="):])
            elif word.startswith("--chroot="):
                chdir = ("--chroot", word[len("--chroot="):])
            index += 1
            continue
        if word in _WRAPPER_VALUE_FLAGS.get(wrapper, ()):
            if index + 1 < count and tokens[index + 1][0] == "word":
                taken = tokens[index + 1][1].text  # type: ignore[attr-defined]
                pieces.extend([word, taken])
                if word in _WRAPPER_CHDIR_OPTIONS.get(wrapper, ()):
                    chdir = (word, taken)
                elif wrapper in _WRAPPER_CHROOT_OPTIONS and word in ("-R", "--chroot"):
                    chdir = ("", taken)  # no separate value rides with a chroot
                index += 2  # the option and its separate value
                continue
            return None, False, pieces, chdir  # the value is missing
        if word in _WRAPPER_FLAGS.get(wrapper, ()):
            pieces.append(word)
            index += 1
            continue
        return None, False, pieces, chdir  # an option that cannot be parsed
    for _ in range(_WRAPPER_LEADING_OPERANDS.get(wrapper, 0)):
        # timeout's duration precedes the program
        if index < count and tokens[index][0] == "word":
            pieces.append(tokens[index][1].text)  # type: ignore[attr-defined]
            index += 1
        else:
            return None, False, pieces, chdir
    return index, False, pieces, chdir


def _attached_split_string(word: str) -> str | None:
    """The value of an attached ``-S``/``--split-string=`` spelling, or
    ``None``: ``-S"git commit"`` holds it without a space."""
    if word.startswith("--split-string=") and len(word) > len("--split-string="):
        return word[len("--split-string="):]
    if word.startswith("-S") and len(word) > 2 and not word.startswith("--"):
        return word[2:]
    return None


def _is_redirect(op: str) -> bool:
    """Any redirect operator, fd-hug included: ``2>``, ``2>&``, ``&>``."""
    return op.lstrip("0123456789&") in ("<", ">", ">>", ">&", "<&", "<>", ">|", "<<", "<<-", "<<<")


def _own_opacity(program: str, args: tuple[str, ...] | list[str]) -> bool:
    """The constructs the parser does not enter (plan 3.6): ``eval`` and
    ``sh -c``. Globs, plain expansions and arithmetic (``$((...))``: it
    evaluates, it runs nothing — :class:`_Word`) are not opaque; they only
    make a ``where`` unknowable. A substitution is none of these: it rides
    each word's own ``subst`` flag (set in :func:`_read_word`, which knows
    the quoting, the ``${...}`` spans and the arithmetic spans), never a
    re-scan of the argument text — an arithmetic span sharing a word must
    not unsee a substitution in it (S3R-hook-runner-C2-1)."""
    if program in _OPAQUE_PROGRAMS:
        return True
    return program == "sh" and "-c" in args


#: Git's global options that take a separate value, for finding the
#: subcommand: they consume the word after them. The one table of its kind
#: (S4R-rules-C2-1): the rules import it, so no second list drifts.
#: ``--exec-path`` takes no separate value; an unlisted global is read as a
#: bare option, and git itself refuses what it does not know.
GIT_GLOBAL_VALUE_OPTIONS = ("-C", "-c", "--git-dir", "--work-tree", "--namespace",
                            "--super-prefix", "--config-env", "--attr-source")

#: The options of a git call whose value is free text — a message or a file
#: the user supplies. A dollar sign or a backtick inside such a value is the
#: value's own text, never a command the rule must chase (S4R-rules-C1-1);
#: the rules that read a git call import this, so no second list drifts.
GIT_FREE_TEXT_OPTIONS = ("-m", "--message", "-F", "--file")


def free_text_value_words(args) -> set[int]:
    """The indexes of ``args`` whose word is a free-text option's value:
    separate (``-m x``) or attached (``-mx``, ``--message=x``)."""
    values: set[int] = set()
    free_next = False
    for index, word in enumerate(args):
        if free_next:
            free_next = False
            values.add(index)
            continue
        if word in GIT_FREE_TEXT_OPTIONS:
            free_next = True
            continue
        if _free_text_attached(word):
            values.add(index)
    return values


def _free_text_attached(word: str) -> bool:
    """An option and its free-text value as one word: ``-mx``, ``--message=x``."""
    if word.startswith("--message=") or word.startswith("--file="):
        return True
    return (
        word.startswith("-") and not word.startswith("--") and len(word) > 2
        and word[1] in "mF"
    )


def _global_args(words: list[_Word]) -> list[_Word]:
    """The words before the subcommand: ``git``/``vsdd -C`` is a global
    option, so a ``-C`` after the verb belongs to that verb (``git switch -C
    develop``, S4R-rules-7) and moves no directory."""
    index = 0
    while index < len(words):
        word = words[index].text
        if word in GIT_GLOBAL_VALUE_OPTIONS:
            index += 2  # the option and its separate value
            continue
        if word.startswith("-") and len(word) > 1:
            index += 1
            continue
        break  # the subcommand itself
    return words[:index]


def _minus_c(words: list[_Word], current: str) -> str | None:
    """``git -C <dir>`` / ``vsdd -C <dir>``, including the fused ``-C<dir>``.
    Every ``-C`` is threaded in order — each resolved against the one before
    it, ``git -C . -C ../.. commit`` climbing as git climbs — and the last
    one stands; ``None`` when no ``-C`` is there, ``""`` when one does not
    resolve (step4-rules-fix-3-C1-5)."""
    at = current
    seen = False
    position = 0
    while position < len(words):
        word = words[position]
        if word.text == "-C" and position + 1 < len(words):
            resolved = _resolve(words[position + 1], at)
            position += 2
        elif word.text.startswith("-C") and len(word.text) > 2:
            resolved = _resolve(_Word(word.text[2:], word.expanded), at)
            position += 1
        else:
            position += 1
            continue
        seen = True
        if not resolved:
            return ""
        at = resolved
    return at if seen else None


def _cd_target(words: list[_Word], where: str, cdpath: bool = False) -> str:
    """The directory a ``cd`` imposes on the later segments: its first operand
    resolved against ``where``. Options (``-L``, ``-P``, ``-e``, ``-@``) are
    skipped, and ``--`` ends them; a bare ``cd`` or an option-only ``cd`` goes
    home, which is `""` here, as is any unknowable target. An absolute operand
    that does not exist at parse time, and any relative one under a CDPATH an
    earlier segment set, name no place the parse can promise
    (step4-rules-fix-3-C1-5)."""
    end_of_options = False
    for word in words:
        if not end_of_options and word.text == "--":
            end_of_options = True
            continue
        if not end_of_options and word.text.startswith("-") and len(word.text) > 1:
            continue  # an option; the operand comes later
        resolved = _resolve(word, where)
        if resolved and cdpath and not resolved.startswith("/"):
            # CDPATH may steer a relative cd anywhere the variable names.
            return ""
        if (resolved and word.text.startswith("/")
                and not os.path.isdir(resolved)):
            # An absolute name that is not a directory now is not a place the
            # parse can promise — a worktree a former segment is about to
            # create included (step4-rules-fix-3-C1-5). A relative name with
            # no CDPATH can only land where it resolves or nowhere.
            return ""
        return resolved
    return ""  # no operand: the shell goes home


def _resolve(target: "_Word | str", current: str) -> str:
    """Absolute ``target`` resolved against ``current``; ``""`` unknowable. An
    expanded word, a tilde or ``-`` names no guessable directory."""
    if isinstance(target, _Word):
        text, expanded = target.text, target.expanded
    else:
        text, expanded = target, False
    if not text or expanded or text.startswith("~") or text == "-":
        return ""
    if not text.startswith("/"):
        if not current:
            return ""
        text = os.path.join(current, text)
    return os.path.normpath(text)
