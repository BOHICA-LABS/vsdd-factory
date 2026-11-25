#!/usr/bin/env python3
"""Checks the restructured design under rewrite/spec/.

Rules it enforces (see rewrite/spec/README.md):
- every thing is listed once in registry.toml (or a registry.d/*.toml addition) and has exactly one file, in the folder for its kind;
- every Markdown file starts with frontmatter whose kind and id match its path;
- every [[kind:id]] link points at a thing that exists;
- every thing that gets built names a build package that exists;
- JSON and TOML files parse;
- no file is larger than MAX_BYTES and no line is longer than MAX_LINE.

Standard library only. Run from anywhere: `python3 rewrite/tools/spec_lint.py [--only <path-prefix>]`.
Exit status 0 means clean.
"""
from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path

SPEC = Path(__file__).resolve().parent.parent / "spec"
MAX_BYTES = 16 * 1024
MAX_LINE = 400

# kind -> (folder, file suffix). Kinds marked BUILT must name a package.
KINDS = {
    "doc": ("", ".md"),
    "topic": ("topics", ".md"),
    "command": ("commands", ".md"),
    "record": ("records", ".md"),
    "schema": ("schemas", ".schema.json"),
    "rule": ("rules", ".md"),
    "agent": ("agents", ".md"),
    "skill": ("skills", ".md"),
    "workflow": ("workflows", ".md"),
    "artifact": ("artifacts", ".md"),
    "table": ("tables", ".toml"),
    "config": ("config", ".md"),
    "packaging": ("packaging", ".md"),
    "package": ("packages", ".md"),
}
BUILT = {"command", "record", "schema", "rule", "agent", "skill", "workflow", "artifact", "table", "config"}
LINK = re.compile(r"\[\[([a-z]+):([A-Za-z0-9_.-]+)(?:#[^\]]*)?\]\]")
ID = re.compile(r"^[A-Za-z0-9][A-Za-z0-9_-]*$")


def path_for(kind: str, ident: str) -> Path:
    folder, suffix = KINDS[kind]
    return (SPEC / folder / f"{ident}{suffix}") if folder else (SPEC / f"{ident}{suffix}")


def read_frontmatter(text: str) -> dict[str, object] | None:
    if not text.startswith("---\n"):
        return None
    block, sep, _ = text[4:].partition("\n---")
    if not sep:
        return None
    data: dict[str, object] = {}
    key = None
    for line in block.split("\n"):
        if not line.strip():
            continue
        if line.startswith(("  - ", "- ")) and key:
            data.setdefault(key, [])
            if isinstance(data[key], list):
                data[key].append(line.split("- ", 1)[1].strip().strip("\"'"))
            continue
        if ":" in line and not line.startswith(" "):
            key, _, value = line.partition(":")
            key = key.strip()
            value = value.strip()
            if value.startswith("[") and value.endswith("]"):
                data[key] = [x.strip().strip("\"'") for x in value[1:-1].split(",") if x.strip()]
            elif value:
                data[key] = value.strip("\"'")
            else:
                data[key] = []
    return data


def main() -> int:
    only = None
    if "--only" in sys.argv:
        only = sys.argv[sys.argv.index("--only") + 1].rstrip("/")
    problems: list[str] = []

    def bad(path: Path | str, message: str) -> None:
        shown = path.relative_to(SPEC) if isinstance(path, Path) else path
        problems.append(f"{shown}: {message}")

    registry_file = SPEC / "registry.toml"
    if not registry_file.exists():
        print("spec/registry.toml is missing")
        return 1
    try:
        registry = tomllib.loads(registry_file.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError as error:
        print(f"registry.toml does not parse: {error}")
        return 1
    # Additions made while several agents work at once live in registry.d/<name>.toml, one file per agent.
    for extra in sorted((SPEC / "registry.d").glob("*.toml")):
        try:
            more = tomllib.loads(extra.read_text(encoding="utf-8"))
        except tomllib.TOMLDecodeError as error:
            print(f"registry.d/{extra.name} does not parse: {error}")
            return 1
        for kind, entries in more.items():
            if not isinstance(entries, dict):
                print(f"registry.d/{extra.name}: [{kind}] must hold one table per id")
                return 1
            for ident, entry in entries.items():
                if ident in registry.setdefault(kind, {}):
                    print(f"registry.d/{extra.name}: {kind}:{ident} is already registered")
                    return 1
                registry[kind][ident] = entry

    things: dict[tuple[str, str], dict] = {}
    for kind, entries in registry.items():
        if kind not in KINDS:
            bad("registry.toml", f"unknown kind [{kind}]")
            continue
        if not isinstance(entries, dict):
            bad("registry.toml", f"[{kind}] must hold one table per id")
            continue
        for ident, entry in entries.items():
            if not ID.match(ident):
                bad("registry.toml", f"{kind}:{ident} is not a valid id")
            if not isinstance(entry, dict) or not entry.get("summary"):
                bad("registry.toml", f"{kind}:{ident} needs a one-line summary")
                entry = entry if isinstance(entry, dict) else {}
            things[(kind, ident)] = entry

    packages = {ident for (kind, ident) in things if kind == "package"}
    for (kind, ident), entry in things.items():
        if kind in BUILT:
            package = entry.get("package")
            if not package:
                bad("registry.toml", f"{kind}:{ident} names no package")
            elif package != "none" and package not in packages:
                bad("registry.toml", f"{kind}:{ident} names package {package}, which is not in the registry")

    def in_scope(path: Path) -> bool:
        return only is None or str(path.relative_to(SPEC)).startswith(only)

    # Every registered thing has its file, and the file says what it is.
    for (kind, ident), entry in things.items():
        path = path_for(kind, ident)
        if not in_scope(path):
            continue
        if not path.exists():
            bad(path, f"missing file for {kind}:{ident}")
            continue
        if path.suffix == ".md":
            front = read_frontmatter(path.read_text(encoding="utf-8"))
            if front is None:
                bad(path, "no frontmatter")
                continue
            if front.get("kind") != kind or front.get("id") != ident:
                bad(path, f"frontmatter says {front.get('kind')}:{front.get('id')}, path says {kind}:{ident}")
            if kind in BUILT and front.get("package") != entry.get("package"):
                bad(path, f"frontmatter package {front.get('package')!r} differs from registry {entry.get('package')!r}")
            if kind not in ("package",) and not front.get("sources"):
                bad(path, "frontmatter lists no sources")

    # Every file belongs to a registered thing; files parse and stay small.
    expected = {path_for(kind, ident) for (kind, ident) in things}
    allowed_extra = {"README.md", "registry.toml", "retired.toml", "source-index.json"}
    for path in sorted(SPEC.rglob("*")):
        if not path.is_file() or not in_scope(path):
            continue
        relative = str(path.relative_to(SPEC))
        if relative.startswith("registry.d/"):
            # Registration files are checked as part of the registry above; here only the line limit applies.
            for number, line in enumerate(path.read_text(encoding="utf-8").split("\n"), 1):
                if len(line) > MAX_LINE:
                    bad(path, f"line {number} is {len(line)} characters (limit {MAX_LINE})")
                    break
            continue
        if relative in allowed_extra or relative.startswith("conflicts/"):
            continue
        if path not in expected:
            bad(path, "file is not in registry.toml")
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            bad(path, "not UTF-8 text")
            continue
        if len(text.encode("utf-8")) > MAX_BYTES:
            bad(path, f"{len(text.encode('utf-8')) // 1024} KB is over the {MAX_BYTES // 1024} KB limit; split it")
        for number, line in enumerate(text.split("\n"), 1):
            if len(line) > MAX_LINE:
                bad(path, f"line {number} is {len(line)} characters (limit {MAX_LINE}); break the table row into a list")
                break
        if path.suffix == ".json":
            try:
                json.loads(text)
            except json.JSONDecodeError as error:
                bad(path, f"does not parse as JSON: {error}")
        if path.suffix == ".toml":
            try:
                tomllib.loads(text)
            except tomllib.TOMLDecodeError as error:
                bad(path, f"does not parse as TOML: {error}")
        if path.suffix in (".md", ".toml"):
            for match in LINK.finditer(text):
                target = (match.group(1), match.group(2))
                if target[0] not in KINDS:
                    bad(path, f"link [[{target[0]}:{target[1]}]] uses an unknown kind")
                elif target not in things:
                    bad(path, f"link [[{target[0]}:{target[1]}]] points at nothing in the registry")

    by_kind: dict[str, int] = {}
    for kind, _ in things:
        by_kind[kind] = by_kind.get(kind, 0) + 1
    print("registry: " + ", ".join(f"{count} {kind}" for kind, count in sorted(by_kind.items())))
    for line in problems:
        print(line)
    print(f"{len(problems)} problem(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
