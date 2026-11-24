"""The shared frontmatter reader: a strict YAML subset (hooks.md 2.4).

Accepted is one ``---`` fenced block at the very top of a file, holding:

- scalars: a string (bare, single- or double-quoted), an integer, ``true`` /
  ``false``, ``null`` (``null``, ``~`` or an empty value), or a bare ISO date
  (``2026-11-24``, read as a ``datetime.date`` — state.md 2.4 lists a date
  among the flat values),
- flow lists of scalars on one line: ``[BC-2.03.007, BC-2.03.008]``,
- block lists of scalars under a key,
- one level of nested mapping (scalars only inside).

Anything else raises :class:`FrontmatterError` with the 1-based line number.
Floats, block scalars (``, ``>``), anchors and aliases, trailing commas in
flow lists, comments on value lines, multi-line values and a second nesting
level are outside the subset and are refused, not misread. Full-line ``#``
comments and blank lines are allowed anywhere in the block.

Refused, not misread, concretely: a plain scalar holding ``#`` (a comment) or
``: `` (a nested mapping on one line); a value starting with an indicator
character (``- ? % @`` or a backtick, then ``! & * | > {``); a YAML 1.1 spelling of a bool, null or
number other than ``true`` / ``false`` / ``null`` / ``~`` / ``-?[0-9]+``
(``yes``, ``no``, ``on``, ``off``, ``True``, ``FALSE``, ``Null``, leading
zeros, ``+5``, ``0x1F``, ``0o17``, ``1_000``, ``12:30``, ``.inf``, ``.nan``);
a block-list item holding a mapping; an empty flow item (``[a,,b]``); a quoted
string with an interior unescaped quote or trailing text, or a double-quoted
escape other than ``\\"`` and ``\\\\``; a mapping line whose colon is not
followed by a space (``key:value``, ``http://x`` as a mapping line). Every
refusal names the line and asks for the value to be quoted, so an agent can
act on it; nothing is read differently from how it looks.

The flat artifact frontmatter of state.md 2.4 (no nested maps, no multi-line
values, one closed key list, at most 200 characters per line) sits inside this
subset; its extra limits belong to the artifact registry (C2), not to the
reader.
"""

from __future__ import annotations

import re
from datetime import date

__all__ = ["FrontmatterError", "parse", "split"]

_OPEN_FENCE = "---"
_INT_RE = re.compile(r"-?(0|[1-9]\d*)")
_ANY_INT_RE = re.compile(r"-?\d+")
_FLOAT_RE = re.compile(r"-?(?:\d+\.\d*(?:[eE][-+]?\d+)?|\.\d+(?:[eE][-+]?\d+)?|\d+[eE][-+]?\d+)")
_DATE_RE = re.compile(r"\d{4}-\d{2}-\d{2}")
_SEXAGESIMAL_RE = re.compile(r"-?\d+(:\d+)+")
_SINGLE_Q_RE = re.compile(r"'(?:[^']|'')*'")
_DOUBLE_Q_RE = re.compile(r'"(?:[^"\\]|\\["\\])*"')
_KEY_RE = re.compile(r"[A-Za-z0-9_.-]+")
_INDICATORS = "-?%@`!&*|>{}"

# YAML 1.1 spells these as booleans, nulls or numbers; only the exact lowercase
# spellings true/false/null/~ are in the subset, so every other case or synonym
# is refused with an instruction to quote.
_YAML11_SPELLINGS = ("yes", "no", "on", "off", "true", "false", "null", "none")
_ACCEPTED_SPELLINGS = ("true", "false", "null")


class FrontmatterError(ValueError):
    """A frontmatter block is outside the subset. ``line`` is 1-based."""

    def __init__(self, message: str, line: int):
        super().__init__(f"frontmatter line {line}: {message}")
        self.line = line


def _indent(line: str) -> int:
    return len(line) - len(line.lstrip(" "))


def _check_tab(line: str, number: int) -> None:
    """Tabs are refused wherever indentation is measured (spaces only)."""
    if "\t" in line[: len(line) - len(line.lstrip())]:
        raise FrontmatterError("tab in indentation", number)


def _scalar(raw: str, line: int) -> object:
    """Parse one scalar as it appears in a value position or a list item."""
    s = raw.strip()
    if not s:
        return None
    if s[0] in "'\"":
        quote = s[0]
        pattern = _SINGLE_Q_RE if quote == "'" else _DOUBLE_Q_RE
        if not pattern.fullmatch(s):
            raise FrontmatterError(
                f"unterminated or badly escaped {quote}-quoted string; escape "
                "interior quotes and end the value with its quote",
                line,
            )
        inner = s[1:-1]
        if quote == "'":
            return inner.replace("''", "'")
        if "\\" in inner.replace('\\"', "").replace("\\\\", ""):
            raise FrontmatterError(
                'the only double-quoted escapes in the subset are \\" and \\\\; '
                "quote the value some other way",
                line,
            )
        return inner.replace('\\"', '"').replace("\\\\", "\\")
    if s.startswith("["):
        raise FrontmatterError("nested flow lists are outside the subset", line)
    if _ANY_INT_RE.fullmatch(s) and not _INT_RE.fullmatch(s):
        raise FrontmatterError(f"{s!r} is not a plain integer in the subset; quote it to keep it as text", line)
    if _SEXAGESIMAL_RE.fullmatch(s):
        raise FrontmatterError(f"{s!r} reads as a base-60 number in YAML; quote it to keep it as text", line)
    if s.lower().lstrip("+-") in (".inf", ".nan"):
        raise FrontmatterError(f"{s!r} is a YAML number spelling; quote it to keep it as text", line)
    if s.startswith("+") or "_" in s or s.lower().startswith(("0x", "0o")):
        raise FrontmatterError(f"{s!r} is a YAML number spelling; quote it to keep it as text", line)
    if s.lower() in _YAML11_SPELLINGS and s not in _ACCEPTED_SPELLINGS:
        raise FrontmatterError(f"{s!r} is a YAML 1.1 spelling; use true, false, null or quote it", line)
    if s == "true":
        return True
    if s == "false":
        return False
    if s in ("null", "~"):
        return None
    if _INT_RE.fullmatch(s):
        return int(s)
    if _FLOAT_RE.fullmatch(s):
        raise FrontmatterError(
            "floats are outside the subset; quote the value as a string", line
        )
    if _DATE_RE.fullmatch(s):
        try:
            return date.fromisoformat(s)
        except ValueError:
            raise FrontmatterError(
                f"{s!r} is not a valid date (YYYY-MM-DD); quote it to keep it as text",
                line,
            ) from None
    if s[0] == "#":
        raise FrontmatterError("a comment is not a value; quote the text", line)
    if " #" in s:
        raise FrontmatterError("a comment on a value line is outside the subset; quote the value", line)
    if ": " in s or s.endswith(":"):
        raise FrontmatterError(f"a scalar holding {': '!r} reads as a mapping; quote the value", line)
    if s[0] in _INDICATORS:
        raise FrontmatterError(
            f"value starts with {s[0]!r}, an indicator character; quote the value",
            line,
        )
    return s


def _flow_list(raw: str, line: int) -> list:
    """Parse a one-line flow list ``[a, b]`` of scalars."""
    if not raw.endswith("]"):
        raise FrontmatterError("flow list is not closed with ]", line)
    inner = raw[1:-1].strip()
    if not inner:
        return []
    items: list[str] = []
    buf = ""
    quote: str | None = None
    for ch in inner:
        if quote:
            if ch == quote:
                quote = None
        elif ch in "'\"":
            quote = ch
        elif ch == ",":
            items.append(buf)
            buf = ""
            continue
        buf += ch
    if quote:
        raise FrontmatterError("unterminated quoted string in flow list", line)
    items.append(buf)
    for index, item in enumerate(items):
        if not item.strip():
            raise FrontmatterError(
                f"empty flow item {index + 1} of {len(items)}: "
                "every item must be a scalar; write [] for an empty list",
                line,
            )
    return [_scalar(item, line) for item in items]


def _meaningful(lines: list[str], start: int, fence_line: int) -> int | None:
    """Index of the next non-blank, non-comment line before the fence."""
    i = start
    while i < fence_line:
        stripped = lines[i].strip()
        if stripped and not stripped.startswith("#"):
            return i
        i += 1
    return None


def _take_block_list(lines: list[str], first: int, fence_line: int) -> tuple[list, int]:
    items: list = []
    base = _indent(lines[first])
    i = first
    while i < fence_line:
        line = lines[i]
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            i += 1
            continue
        _check_tab(line, i + 1)
        ind = _indent(line)
        is_item = stripped == "-" or stripped.startswith("- ")
        if ind < base or (i > first and ind == 0 and not is_item):
            break
        if ind > base:
            raise FrontmatterError("unexpected indentation in a block list", i + 1)
        if not (stripped == "-" or stripped.startswith("- ")):
            break
        rest = stripped[1:].strip()
        if not rest:
            raise FrontmatterError("a list item must be a scalar in the subset", i + 1)
        if ": " in rest or rest.endswith(":"):
            raise FrontmatterError(
                "a mapping inside a list item is outside the subset; "
                "make the item a quoted scalar",
                i + 1,
            )
        items.append(_scalar(rest, i + 1))
        i += 1
    return items, i


def _take_nested_map(lines: list[str], first: int, fence_line: int) -> tuple[dict, int]:
    sub: dict = {}
    base = _indent(lines[first])
    i = first
    while i < fence_line:
        line = lines[i]
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            i += 1
            continue
        _check_tab(line, i + 1)
        ind = _indent(line)
        if ind <= _indent(lines[first]) - 1 or (i > first and ind == 0):
            break
        if ind > base:
            raise FrontmatterError(
                "a second nesting level is outside the subset", i + 1
            )
        key, value = _split_key_value(stripped, i + 1)
        if key in sub:
            raise FrontmatterError(f"duplicate key {key!r}", i + 1)
        if value is _NO_VALUE:
            # A nested key may not open a second level; peek to name the
            # offence precisely when deeper content follows.
            deeper = _meaningful(lines, i + 1, fence_line)
            if deeper is not None and _indent(lines[deeper]) > ind:
                raise FrontmatterError(
                    "a second nesting level is outside the subset", deeper + 1
                )
            raise FrontmatterError(f"nested key {key!r} needs a scalar value", i + 1)
        sub[key] = (
            _flow_list(value, i + 1) if value.startswith("[") else _scalar(value, i + 1)
        )
        i += 1
    return sub, i


_NO_VALUE = object()


def _split_key_value(stripped: str, line: int) -> tuple[str, str | object]:
    """Split ``key: value``; refuse ``key:value`` (a YAML scalar), never a misread.

    Returns ``(key, value)``, or ``(key, _NO_VALUE)`` when the value is empty.
    """
    key, sep, rest = stripped.partition(":")
    key = key.strip()
    if not sep or not _KEY_RE.fullmatch(key):
        raise FrontmatterError(f"expected 'key: value', got {stripped!r}", line)
    if rest == "":
        return key, _NO_VALUE
    if not rest.startswith(" "):
        raise FrontmatterError(
            f"expected 'key: value' with a space after the colon, got {stripped!r}; "
            "quote the value to keep it as text",
            line,
        )
    return key, rest.strip()


def _parse_block(lines: list[str], fence_line: int) -> dict:
    meta: dict = {}
    i = 1  # lines[0] is the opening fence
    while i < fence_line:
        line = lines[i]
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            i += 1
            continue
        if "\t" in line[: len(line) - len(line.lstrip())]:
            raise FrontmatterError("tab in indentation", i + 1)
        if _indent(line) != 0:
            raise FrontmatterError("unexpected indentation at top level", i + 1)
        if stripped == _OPEN_FENCE or stripped.startswith(_OPEN_FENCE):
            raise FrontmatterError("unbalanced document fence", i + 1)
        key, value = _split_key_value(stripped, i + 1)
        if key in meta:
            raise FrontmatterError(f"duplicate key {key!r}", i + 1)
        if value is not _NO_VALUE:
            if value.startswith("#"):
                # The whole value is a comment: YAML reads an empty value.
                meta[key] = None
                i += 1
                continue
            meta[key] = (
                _flow_list(value, i + 1) if value.startswith("[") else _scalar(value, i + 1)
            )
            i += 1
            continue
        nxt = _meaningful(lines, i + 1, fence_line)
        if nxt is None:
            meta[key] = None
            i += 1
            continue
        nxt_line = lines[nxt]
        nxt_stripped = nxt_line.strip()
        # A list item may sit at the key's own indent or deeper; a mapping may
        # only nest deeper; any other zero-indent line is the next top-level
        # key, which leaves the empty value null.
        if nxt_stripped == "-" or nxt_stripped.startswith("- "):
            meta[key], i = _take_block_list(lines, nxt, fence_line)
        elif _indent(nxt_line) > 0 and ":" in nxt_stripped:
            meta[key], i = _take_nested_map(lines, nxt, fence_line)
        elif _indent(nxt_line) == 0:
            meta[key] = None
            i += 1
            continue
        else:
            raise FrontmatterError(
                f"unsupported construct under {key!r}: {nxt_stripped!r}", nxt + 1
            )
    return meta


def _strip_bom(text: str) -> str:
    return text[1:] if text.startswith("﻿") else text


def _split_index(text: str) -> tuple[dict, int | None]:
    """The parsed block and the character offset just past the closing fence."""
    kept = text.splitlines(keepends=True)
    lines = [line.rstrip("\r\n") for line in kept]
    if not lines or lines[0].rstrip() != _OPEN_FENCE:
        return {}, None
    close = next((i for i in range(1, len(lines)) if lines[i].rstrip() == _OPEN_FENCE), -1)
    if close == -1:
        raise FrontmatterError("frontmatter block is not closed with ---", 1)
    # Recount over the original terminators (LF, CRLF or CR), so the body is
    # the file's own bytes from after the closing fence.
    offset = sum(len(line) for line in kept[: close + 1])
    return _parse_block(lines, close), offset


def parse(text: str) -> dict:
    """Parse the frontmatter block of ``text``; ``{}`` when there is none."""
    meta, _ = _split_index(_strip_bom(text))
    return meta


def split(text: str) -> tuple[dict, str]:
    """Return ``(frontmatter, body)``; the body starts after the closing fence.

    The body is cut from the text itself, so a trailing newline and CRLF line
    endings survive exactly as the file carries them.
    """
    text = _strip_bom(text)
    meta, offset = _split_index(text)
    if offset is None:
        return meta, text
    return meta, text[offset:]
