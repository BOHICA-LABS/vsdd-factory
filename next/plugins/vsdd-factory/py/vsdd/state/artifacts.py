"""The artifact registry reader and the checks one file must pass
(state-core.md 3.5; rule:artifacts; topic:artifact-rules).

``vsdd.policy.load("artifacts")`` is the single definition. This module
compiles its rows once per process and checks one file's text at write time
(hook rule ``artifacts``) or a whole tree at once (``vsdd validate``). Reason
codes are those of spec rules/artifacts.md except ``artifact_frozen`` (which
needs the record stores, built later), plus ``criterion_contract``.

Registry-cell grammar, decided in the plan (decision 7):

- a ``glob`` cell is alternatives separated by ``", "`` outside parentheses;
  ``<word>`` is ``*``; a trailing ``.(a, b)`` expands; an alternative with no
  ``/`` takes the first alternative's directory;
- a frontmatter type is ``str``, ``int``, ``bool``, ``date``, ``ref:<kind>``
  or ``list[<type>]``, with a trailing ``?`` for optional, or a list of
  allowed values; ``""`` or a string holding a space is an untyped optional
  key; a string ``frontmatter`` cell means no frontmatter to check;
- typed keys without ``?`` are required; null counts as absent.
"""

from __future__ import annotations

import functools
import re
from dataclasses import dataclass, field
from pathlib import Path

from ..frontmatter import FrontmatterError, split as split_frontmatter
from ..globs import glob_regex
from ..policy import load as load_policy

__all__ = []  # the facade lists only state-api names; mount.py exports detect

_CONSTANT_REF_PATTERNS = {
    "capability": r"CAP-\d{3}",
    "subsystem": r"SS-\d{2}",
    "screen": r"SCR-\d{3}",
}

_EXT_RE = re.compile(r"\.([A-Za-z0-9]+)$")
_PAREN_ALT_RE = re.compile(r"^(.*)\.\(([^()]*)\)$")
_WORD_PLACEHOLDER_RE = re.compile(r"<[^<>]*>")
_AC_HEADING_RE = re.compile(r"^(AC-\d+)\s*(?:\(([^()]*)\))?$")


class RegistryError(Exception):
    """A registry row is outside the decided grammar; names the row."""


class ArtifactError(Exception):
    """An artifact is absent or unreadable."""


@dataclass(frozen=True)
class Problem:
    """One reason a file is not a valid artifact of its kind."""

    rel: str
    code: str
    message: str
    severity: str  # "error" | "warning"


@dataclass(frozen=True)
class Artifact:
    kind: str
    id: str
    rel: str
    frontmatter: dict
    body: str


@dataclass
class Kind:
    """One registry row, compiled."""

    name: str
    raw: dict
    patterns: tuple = ()
    raw_glob: str = ""
    id_re: re.Pattern | None = None
    dir_cell: str = ""
    ext: str = ".md"
    fm_spec: dict | None = None
    headings: tuple = ()
    max_bytes: int | None = None
    forbid: tuple = ()
    tables: bool = False

    def expected_rel(self, ident: str) -> str | None:
        """The path the kind's ``dir`` cell derives from an identifier: the
        rendered directory replaces the kind glob's own wildcard directory
        segment (``E-*`` for stories, ``*`` for contracts)."""
        if self.id_re is None:
            return None
        match = self.id_re.fullmatch(ident)
        if match is None:
            return None
        directory = self.dir_cell
        for index, group in enumerate(match.groups(""), start=1):
            directory = directory.replace(f"{{{index}}}", group)
        base = self.raw_glob.split(",", 1)[0].strip()
        segments = _WORD_PLACEHOLDER_RE.sub("*", base).split("/")
        dirs, file_part = segments[:-1], segments[-1]
        wild = [index for index, segment in enumerate(dirs) if "*" in segment]
        if wild:
            dirs[wild[-1]] = directory
        else:
            dirs.append(directory)
        del file_part
        return "/".join(dirs) + f"/{ident}{self.ext}"


# ----------------------------------------------------------- compilation ---

def _alternatives(cell: str, where: str) -> list[str]:
    """Split on ``", "`` outside parentheses, expand a trailing ``.(a, b)``."""
    parts: list[str] = []
    depth = 0
    current = ""
    for char in cell:
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
        elif char == "," and depth == 0:
            parts.append(current)
            current = ""
            continue
        current += char
    if current.strip():
        parts.append(current)
    expanded: list[str] = []
    for part in parts:
        part = part.strip()
        if not part:
            raise RegistryError(f"{where}: an empty alternative in glob {cell!r}")
        paren = _PAREN_ALT_RE.match(part)
        if paren:
            prefix, options = paren.group(1), paren.group(2)
            expanded.extend(f"{prefix}.{option.strip()}" for option in options.split(","))
        else:
            expanded.append(part)
    first = expanded[0]
    directory = first.rsplit("/", 1)[0] if "/" in first else ""
    filled: list[str] = []
    for alternative in expanded:
        if "/" not in alternative and directory:
            filled.append(f"{directory}/{alternative}")
        else:
            filled.append(alternative)
    return filled


@functools.lru_cache(maxsize=1)
def registry() -> "Registry":
    """The compiled registry, built once from the shipped policy table."""
    raw = load_policy("artifacts")
    kinds: dict[str, Kind] = {}
    unknown_policy = "block"
    for name, row in raw.items():
        if name == "story-kind":
            continue
        if name == "_unknown":
            if isinstance(row, dict) and isinstance(row.get("policy"), str):
                unknown_policy = row["policy"]
            continue
        if not isinstance(row, dict):
            raise RegistryError(f"[{name}] is not a table")
        kinds[name] = _compile_kind(name, row)
    # A ref: target with no pattern anywhere is a compile-time fault: left
    # alone it would surface later as a RegistryError from the middle of a
    # check, or as a frontmatter problem blamed on the agent's file. The
    # error names the row and the key holding the bad cell, not only the
    # target (plan 3.5: the RegistryError names the row).
    for kind in kinds.values():
        for key, target in _ref_targets(kind):
            try:
                _ref_pattern(kinds, target)
            except RegistryError as exc:
                raise RegistryError(f"[{kind.name}].{key}: {exc}") from exc
    return Registry(raw=raw, kinds=kinds, unknown_policy=unknown_policy)


@dataclass
class Registry:
    raw: dict
    kinds: dict[str, Kind]
    unknown_policy: str

    @property
    def territory(self) -> frozenset[str]:
        """The first directories under ``.factory/`` the globs name."""
        found: set[str] = set()
        for kind in self.kinds.values():
            for alternative in _alternatives(kind.raw_glob, f"[{kind.name}]"):
                parts = alternative.split("/")
                if len(parts) >= 2 and parts[0] == ".factory":
                    found.add(parts[1])
        return frozenset(found)


_SCALAR_TYPES = ("str", "int", "bool", "date")


def _validate_type_cell(name: str, key: str, cell) -> None:
    """One frontmatter cell against the decision-7 grammar, at compile time.

    A cell a check cannot interpret must fail the registry build, not surface
    later as a ``RegistryError`` in the middle of a write, or as a frontmatter
    problem blamed on the agent's file.
    """
    where = f"[{name}].{key}"
    if isinstance(cell, list):
        if not cell or not all(isinstance(item, str) for item in cell):
            raise RegistryError(
                f"{where}: an allowed-values cell is a non-empty list of strings"
            )
        return
    if not isinstance(cell, str):
        raise RegistryError(f"{where}: the cell is not a type, an allowed-values list or a string")
    if cell == "" or " " in cell:
        return  # untyped optional key: presence only
    descriptor = cell[:-1] if cell.endswith("?") else cell
    if descriptor in _SCALAR_TYPES:
        return
    if descriptor.startswith("ref:"):
        if not descriptor[len("ref:") :]:
            raise RegistryError(f"{where}: a ref: names a kind")
        return
    if descriptor.startswith("list[") and descriptor.endswith("]"):
        inner = descriptor[len("list[") : -1]
        if inner in _SCALAR_TYPES or (inner.startswith("ref:") and len(inner) > len("ref:")):
            return
        raise RegistryError(f"{where}: unknown list member type {inner!r}")
    raise RegistryError(f"{where}: unknown registry type {descriptor!r}")


def _compile_kind(name: str, row: dict) -> Kind:
    glob_cell = row.get("glob")
    if not isinstance(glob_cell, str):
        raise RegistryError(f"[{name}] has no glob cell")
    alternatives = _alternatives(glob_cell, f"[{name}]")
    patterns = tuple(glob_regex(_WORD_PLACEHOLDER_RE.sub("*", alt)) for alt in alternatives)

    ext = ".md"
    ext_match = _EXT_RE.search(alternatives[0].rsplit("/", 1)[-1])
    if ext_match:
        ext = ext_match.group(0)

    id_cell = row.get("id")
    id_re = None
    if isinstance(id_cell, str) and id_cell:
        try:
            id_re = re.compile(id_cell)
        except re.error as exc:
            raise RegistryError(f"[{name}]: bad id pattern {id_cell!r}: {exc}") from exc

    fm_cell = row.get("frontmatter", None)
    if isinstance(fm_cell, dict):
        fm_spec = fm_cell
        for fm_key, cell in fm_spec.items():
            _validate_type_cell(name, fm_key, cell)
    elif fm_cell is None or isinstance(fm_cell, str):
        fm_spec = None  # a string cell means no frontmatter to check
    else:
        raise RegistryError(f"[{name}]: frontmatter must be a table or a string")

    headings = tuple(row.get("headings", ()))
    forbid = tuple(re.compile(p) for p in row.get("forbid", ()))
    max_bytes = row.get("max_bytes")
    if max_bytes is not None and not isinstance(max_bytes, int):
        raise RegistryError(f"[{name}]: max_bytes must be an integer")
    return Kind(
        name=name,
        raw=row,
        patterns=patterns,
        raw_glob=glob_cell,
        id_re=id_re,
        dir_cell=str(row.get("dir", "")),
        ext=ext,
        fm_spec=fm_spec,
        headings=headings,
        max_bytes=max_bytes,
        forbid=forbid,
        tables=bool(row.get("tables", False)),
    )


def _ref_pattern(kinds: dict[str, Kind], target: str) -> re.Pattern:
    """The syntax pattern of a ``ref:<kind>`` target (decision 8)."""
    kind = kinds.get(target)
    if kind is not None and kind.id_re is not None:
        return kind.id_re
    constant = _CONSTANT_REF_PATTERNS.get(target)
    if constant is not None:
        return re.compile(constant)
    raise RegistryError(f"ref:{target}: a kind with no id row and no constant pattern")


# -------------------------------------------------------------- the kinds ---

def kind_of(rel: str) -> str | None:
    """The kind of a project-relative POSIX path: a kind's name, ``"_unknown"``
    under an artifact directory with no matching kind, ``None`` outside the
    artifact directories (records, ``factory.toml``, ``local/``)."""
    registry_ = registry()
    posix = Path(rel).as_posix()
    if posix.startswith("./"):
        posix = posix[2:]
    parts = posix.split("/")
    if len(parts) < 2 or parts[0] != ".factory":
        return None
    if parts[1] not in registry_.territory:
        return None
    for name, kind in registry_.kinds.items():
        if any(pattern.fullmatch(posix) for pattern in kind.patterns):
            return name
    return "_unknown"


def path_for(ident: str) -> tuple[str, str] | None:
    """``(kind, rel)`` an identifier derives to, or None for an unknown shape."""
    registry_ = registry()
    for name, kind in registry_.kinds.items():
        rel = kind.expected_rel(ident)
        if rel is not None:
            return name, rel
    return None


def load_artifact(root, ident_or_rel) -> Artifact:
    """Read one artifact by identifier or by project-relative path."""
    root = Path(root)
    registry_ = registry()
    given = str(ident_or_rel)
    if given.startswith(".factory"):
        rel = Path(given).as_posix()
        kind_name = kind_of(rel)
        if kind_name in (None, "_unknown"):
            raise ArtifactError(f"{rel} matches no artifact kind")
    else:
        hit = path_for(given)
        if hit is None:
            raise ArtifactError(f"no artifact kind matches identifier {given!r}")
        kind_name, rel = hit
    path = root / rel
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ArtifactError(f"cannot read {rel}: {exc}") from exc
    try:
        frontmatter, body = split_frontmatter(text)
    except FrontmatterError as exc:
        raise ArtifactError(f"{rel}: {exc}") from exc
    kind = registry_.kinds[kind_name]
    ident = frontmatter.get("id") or path.stem
    if kind.id_re is not None:
        ident = path.stem
    return Artifact(kind=kind_name, id=str(ident), rel=rel, frontmatter=frontmatter, body=body)


# ---------------------------------------------------------------- checking ---

def _type_problem(spec: object, value, key: str) -> str | None:
    """One key against its registry type cell; None when it passes."""
    if isinstance(spec, list):
        if value not in spec:
            return f"{key} must be one of {'|'.join(str(s) for s in spec)}, got {value!r}"
        return None
    descriptor = str(spec)
    if descriptor == "" or " " in descriptor:
        return None  # untyped optional key: presence only
    optional = descriptor.endswith("?")
    base = descriptor[:-1] if optional else descriptor
    if value is None:
        return None if optional else f"{key} is required"
    if base == "str":
        if not isinstance(value, str):
            return f"{key} must be a string, got {type(value).__name__}"
    elif base == "int":
        if isinstance(value, bool) or not isinstance(value, int):
            return f"{key} must be an integer, got {type(value).__name__}"
    elif base == "bool":
        if not isinstance(value, bool):
            return f"{key} must be true or false, got {value!r}"
    elif base == "date":
        if not hasattr(value, "year"):
            return f"{key} must be a date (YYYY-MM-DD), got {value!r}"
    elif base.startswith("ref:"):
        if not isinstance(value, str):
            return f"{key} must be a {base} identifier, got {type(value).__name__}"
    elif base.startswith("list["):
        inner = base[len("list[") : -1]
        if not isinstance(value, list):
            return f"{key} must be a list, got {type(value).__name__}"
        for item in value:
            inner_problem = _type_problem(inner, item, key)
            if inner_problem:
                return inner_problem
    else:
        return f"{key}: unknown registry type {descriptor!r}"
    return None


def _ref_targets(kind: Kind) -> list[tuple[str, str]]:
    """``(key, target_kind)`` for every ``ref:`` and ``list[ref:...]`` cell."""
    out: list[tuple[str, str]] = []
    for key, spec in (kind.fm_spec or {}).items():
        if not isinstance(spec, str) or spec == "" or " " in spec:
            continue
        base = spec[:-1] if spec.endswith("?") else spec
        if base.startswith("list[ref:"):
            out.append((key, base[len("list[ref:") : -1]))
        elif base.startswith("ref:"):
            out.append((key, base[len("ref:") :]))
    return out


def _required_key_missing(kind: Kind, key: str) -> bool:
    """The registry cell marks ``key`` required (typed, no ``?``)."""
    cell = (kind.fm_spec or {}).get(key)
    if isinstance(cell, list):
        return True
    if not isinstance(cell, str) or cell == "" or " " in cell:
        return False
    return not cell.endswith("?")


def _check_identity(kind: Kind, rel: str, frontmatter: dict) -> list[Problem]:
    if kind.id_re is None:
        return []
    problems: list[Problem] = []
    ident = frontmatter.get("id")
    if ident is None:
        if _required_key_missing(kind, "id"):
            problems.append(
                Problem(rel, "id_mismatch", f"{rel}: no id in the frontmatter", "error")
            )
        return problems
    ident = str(ident)
    expected = kind.expected_rel(ident)
    if expected is None:
        problems.append(
            Problem(
                rel,
                "id_mismatch",
                f"{ident!r} does not match the {kind.name} identifier pattern {kind.id_re.pattern!r}",
                "error",
            )
        )
    elif expected != Path(rel).as_posix():
        problems.append(
            Problem(
                rel,
                "id_mismatch",
                f"a {kind.name} with id {ident} belongs at {expected}, not {rel}",
                "error",
            )
        )
    return problems


def _headings(body: str) -> list[tuple[int, str]]:
    """``(#, text)`` of the heading lines outside code fences."""
    found: list[tuple[int, str]] = []
    fenced = False
    for line in body.splitlines():
        stripped = line.strip()
        if stripped.startswith("```"):
            fenced = not fenced
            continue
        if fenced or not stripped.startswith("#"):
            continue
        level = len(stripped) - len(stripped.lstrip("#"))
        text = stripped[level:].strip()
        if text:
            found.append((level, text))
    return found


#: A body line is at most this many characters (topics/artifact-rules.md,
#: Size caps; decision 10 makes an over-long line a ``size_cap``).
LINE_LIMIT = 1000


def _line_problem(kind: Kind, rel: str, body: str) -> list[Problem]:
    """Body lines over ``LINE_LIMIT`` characters are a ``size_cap``."""
    if kind.max_bytes is None:
        return []
    for number, line in enumerate(body.splitlines(), start=1):
        if len(line) > LINE_LIMIT:
            return [
                Problem(
                    rel,
                    "size_cap",
                    f"{rel} line {number} is {len(line):,} characters; "
                    f"a body line is at most {LINE_LIMIT:,}. Break it or split the file.",
                    "error",
                )
            ]
    return []


def _size_problem(kind: Kind, rel: str, text: str, config, old_size) -> list[Problem]:
    cap = kind.max_bytes
    if cap is None:
        return []
    data = getattr(config, "data", config) or {}
    caps = data.get("caps", {}) if isinstance(data, dict) else {}
    # A waiver names the path with or without the `.factory/` prefix: the
    # spec's example is relative to the state directory (finding S3R-21).
    inside_state = rel[len(".factory/") :] if rel.startswith(".factory/") else rel
    for pattern in caps.get("waived", []) if isinstance(caps, dict) else []:
        compiled = glob_regex(str(pattern))
        if compiled.fullmatch(rel) or (inside_state != rel and compiled.fullmatch(inside_state)):
            return []
    raised = caps.get(kind.name)
    if isinstance(raised, int) and raised > 0:
        cap = raised  # the validated value is the cap (finding SL1-49)
    size = len(text.encode("utf-8"))
    if size <= cap or (old_size is not None and size <= old_size):
        return []
    return [
        Problem(
            rel,
            "size_cap",
            f"{rel} would be {size:,} bytes; the cap is {cap:,}. Make the file smaller or split it.",
            "error",
        )
    ]


def _check_criteria(rel: str, frontmatter: dict, body: str) -> list[Problem]:
    problems: list[Problem] = []
    seen: dict[str, int] = {}
    contracts = frontmatter.get("contracts") or []
    acs: list[tuple[str, str]] = []
    for level, text in _headings(body):
        if level != 3:
            continue
        if not text.startswith("AC-"):
            continue
        match = _AC_HEADING_RE.match(text)
        if match:
            acs.append((match.group(1), match.group(2) or ""))
        else:
            # `### AC-1: Title (...)` escapes both the contract check and the
            # uniqueness check silently, so it is flagged outright.
            problems.append(
                Problem(
                    rel,
                    "criterion_contract",
                    f"{text!r} is not an '### AC-N (<contract>)' heading",
                    "error",
                )
            )
    for name, _paren in acs:
        seen[name] = seen.get(name, 0) + 1
    for name, count in sorted(seen.items()):
        if count > 1:
            problems.append(
                Problem(
                    rel,
                    "criterion_contract",
                    f"{name} appears {count} times; acceptance criteria are unique",
                    "error",
                )
            )
    if contracts:
        listed = [str(item) for item in contracts]
        for name, paren in acs:
            tokens = re.findall(r"[A-Za-z]+[-0-9.A-Za-z]*", paren)
            if not any(token in listed for token in tokens):
                problems.append(
                    Problem(
                        rel,
                        "criterion_contract",
                        f"{name} names none of the story's contracts ({', '.join(listed)})",
                        "error",
                    )
                )
    return problems


def _check_tables(kind: Kind, rel: str, body: str) -> list[Problem]:
    problems: list[Problem] = []
    rows: list[int] = []
    fenced = False
    for line in body.splitlines():
        stripped = line.strip()
        if stripped.startswith("```"):
            fenced = not fenced
            continue
        if fenced:
            continue
        is_row = stripped.startswith("|") and stripped.endswith("|")
        if not is_row:
            if rows:
                header_cells = rows[0]
                for index, cells in enumerate(rows[1:], start=1):
                    if cells != header_cells:
                        problems.append(
                            Problem(
                                rel,
                                "table_malformed",
                                f"table row {index} has {cells} cells; its header has {header_cells}",
                                "error",
                            )
                        )
                rows = []
            continue
        inner = stripped[1:-1]
        rows.append(len(inner.split("|")))
    if rows:
        header_cells = rows[0]
        for index, cells in enumerate(rows[1:], start=1):
            if cells != header_cells:
                problems.append(
                    Problem(
                        rel,
                        "table_malformed",
                        f"table row {index} has {cells} cells; its header has {header_cells}",
                        "error",
                    )
                )
    return problems


def check_text(rel, text: str, *, config, references=None, old_size: int | None = None) -> list[Problem]:
    """The write-time check of one project-relative POSIX path.

    ``references`` is a project root: give it to also report
    ``dangling_reference`` for references to identified kinds.
    """
    rel = Path(rel).as_posix()
    registry_ = registry()
    kind_name = kind_of(rel)
    if kind_name is None:
        return []
    if kind_name == "_unknown":
        severity = "error" if registry_.unknown_policy == "block" else "warning"
        return [
            Problem(
                rel,
                "unknown_artifact_path",
                f"{rel} is under the state directory but matches no artifact kind",
                severity,
            )
        ]
    kind = registry_.kinds[kind_name]
    problems: list[Problem] = []

    if kind.fm_spec is None:
        frontmatter: dict = {}
        body = text
    else:
        try:
            frontmatter, body = split_frontmatter(text)
        except FrontmatterError as exc:
            problems.append(Problem(rel, "frontmatter_invalid", str(exc), "error"))
            frontmatter, body = {}, text
        else:
            problems.extend(_check_frontmatter_keys(kind, rel, frontmatter))
        problems.extend(_check_identity(kind, rel, frontmatter))

    problems.extend(_check_required_headings(kind, rel, body))
    problems.extend(_check_criteria_if_any(kind, rel, frontmatter, body))
    problems.extend(_size_problem(kind, rel, text, config, old_size))
    problems.extend(_line_problem(kind, rel, body))
    problems.extend(_check_forbidden(kind, rel, body))
    if kind.tables:
        problems.extend(_check_tables(kind, rel, body))
    problems.extend(
        _check_references(registry_, kind, rel, frontmatter, references)
    )
    return problems


def _check_frontmatter_keys(kind: Kind, rel: str, frontmatter: dict) -> list[Problem]:
    problems: list[Problem] = []
    spec = kind.fm_spec or {}
    for key in frontmatter:
        if key not in spec:
            problems.append(
                Problem(
                    rel,
                    "frontmatter_invalid",
                    f"{key}: unknown key for a {kind.name}",
                    "error",
                )
            )
    for key, cell in spec.items():
        present = key in frontmatter
        value = frontmatter.get(key)
        if not present and _required_key_missing(kind, key):
            problems.append(Problem(rel, "frontmatter_invalid", f"{key} is required", "error"))
            continue
        if present:
            problem = _type_problem(cell, value, key)
            if problem:
                problems.append(Problem(rel, "frontmatter_invalid", problem, "error"))
    return problems


def _check_required_headings(kind: Kind, rel: str, body: str) -> list[Problem]:
    level_two = {text for level, text in _headings(body) if level == 2}
    problems: list[Problem] = []
    for heading in kind.headings:
        if heading not in level_two:
            problems.append(
                Problem(rel, "heading_missing", f"no '## {heading}' heading", "error")
            )
    return problems


def _check_criteria_if_any(kind: Kind, rel: str, frontmatter: dict, body: str) -> list[Problem]:
    if "Acceptance criteria" not in kind.headings:
        return []
    return _check_criteria(rel, frontmatter, body)


def _check_forbidden(kind: Kind, rel: str, body: str) -> list[Problem]:
    problems: list[Problem] = []
    for pattern in kind.forbid:
        match = pattern.search(body)
        if match:
            problems.append(
                Problem(
                    rel,
                    "forbidden_pattern",
                    f"{rel} holds the forbidden pattern {pattern.pattern!r}",
                    "error",
                )
            )
    return problems


def _check_references(registry_: Registry, kind: Kind, rel: str, frontmatter: dict, references) -> list[Problem]:
    problems: list[Problem] = []
    for key, target in _ref_targets(kind):
        pattern = _ref_pattern(registry_.kinds, target)  # compiled clean above
        value = frontmatter.get(key)
        items = value if isinstance(value, list) else ([] if value is None else [value])
        for item in items:
            ident = str(item)
            if not pattern.fullmatch(ident):
                problems.append(
                    Problem(
                        rel,
                        "reference_malformed",
                        f"{key}: {ident!r} is not a well-formed {target} identifier",
                        "error",
                    )
                )
                continue
            if references is None:
                continue
            target_kind = registry_.kinds.get(target)
            if target_kind is None or target_kind.id_re is None:
                continue  # existence is checked only for kinds with an id row
            expected = target_kind.expected_rel(ident)
            if expected is not None and not (Path(references) / expected).exists():
                problems.append(
                    Problem(
                        rel,
                        "dangling_reference",
                        f"{key}: {ident} does not exist ({expected})",
                        "error",
                    )
                )
    return problems


# ------------------------------------------------------------------ trees ---

def tree_rels(root) -> list[str]:
    """Every file under ``.factory/``, as project-relative POSIX paths."""
    base = Path(root) / ".factory"
    rels: list[str] = []
    if not base.is_dir():
        return rels
    for path in sorted(base.rglob("*")):
        if path.is_file():
            rels.append(path.relative_to(Path(root)).as_posix())
    return rels


def _tree_rels(root) -> list[str]:  # kept for callers inside the package
    return tree_rels(root)


def check_paths(root, rels=None, *, config, references=True):
    """Check files of the state tree; ``rels=None`` is the whole tree.

    A file in the artifact territory that cannot be read or decoded is not
    skipped silently: it is reported as an ``unreadable`` error, so a flush
    can never commit it unchecked.
    """
    root = Path(root)
    if rels is None:
        rels = tree_rels(root)
    problems: list[Problem] = []
    for rel in rels:
        rel_posix = Path(rel).as_posix()
        if kind_of(rel_posix) is None:
            continue  # outside the artifact territories: nothing to check
        try:
            text = (root / rel).read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError) as exc:
            problems.append(
                Problem(
                    rel_posix,
                    "unreadable",
                    f"{rel_posix} cannot be read or decoded: {exc}",
                    "error",
                )
            )
            continue
        problems.extend(
            check_text(
                rel_posix,
                text,
                config=config,
                references=root if references is True else (references or None),
            )
        )
    return problems


def changed_paths(root) -> list[str]:
    """Modified and untracked files of the state worktree, as project-relative
    paths starting ``.factory/``.

    Read from ``git status --porcelain=v1 -z``: NUL-separated records are
    never C-quoted, so dotfiles, spaces and non-ASCII names come back whole.
    """
    root = Path(root)
    from .gitrun import git

    raw = git(root / ".factory", "status", "--porcelain=v1", "-z", "-uall")
    records = raw.split("\0")
    changed: list[str] = []
    index = 0
    while index < len(records):
        record = records[index]
        index += 1
        if not record:
            continue
        flag, name = record[:2], record[3:]
        if flag[0] in ("R", "C") and index < len(records):
            index += 1  # the paired original path follows each R/C record
        if flag == "??" or any(mark in flag for mark in "MATC"):
            changed.append(".factory/" + name)
    return changed
