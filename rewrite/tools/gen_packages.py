#!/usr/bin/env python3
"""Generates the build package briefs in rewrite/spec/packages/ and their registry entries.

A brief is built from two sources, so it cannot disagree with either:
- the build plan tables in section 9 of the blueprint (what the package delivers and depends on), plus any
  package added since, registered in rewrite/spec/registry.d/ (see added_packages);
- rewrite/spec/registry.toml and registry.d/ (every thing whose `package` is this package).

Text a person or agent adds under "## Notes" is kept when the brief is regenerated.
Standard library only. Run from anywhere: `python3 rewrite/tools/gen_packages.py`.
"""
from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

REWRITE = Path(__file__).resolve().parent.parent
SPEC = REWRITE / "spec"
BLUEPRINTS = [REWRITE / "BLUEPRINT.md", REWRITE / "history" / "design-v1" / "BLUEPRINT.md"]
PACKAGE_ID = re.compile(r"^[A-Z]{1,2}\d+[a-z]?$")
START, END = "<!-- generated: start -->", "<!-- generated: end -->"
REG_START, REG_END = "# generated: packages start", "# generated: packages end"


def blueprint_text() -> str:
    for path in BLUEPRINTS:
        if path.exists():
            return path.read_text(encoding="utf-8")
    raise SystemExit("cannot find the blueprint")


def parse_plan() -> list[dict]:
    text = blueprint_text()
    start = text.index("\n## 9. ")
    end = text.index("\n## 10. ", start)
    packages: list[dict] = []
    section = "9"
    for line in text[start:end].split("\n"):
        heading = re.match(r"^### (9\.\d+)", line)
        if heading:
            section = heading.group(1)
            continue
        if not line.startswith("|"):
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) < 3 or not PACKAGE_ID.match(cells[0]):
            continue
        ident, title = cells[0], cells[1]
        delivers, depends = (cells[2], cells[3]) if len(cells) >= 4 else ("", cells[2])
        packages.append({"id": ident, "title": title, "delivers": delivers, "depends_raw": depends, "section": section})
    return packages


def expand_depends(raw: str, known: set[str]) -> tuple[list[str], str]:
    found: list[str] = []
    for match in re.finditer(r"\b([A-Z]{1,2})(\d+) to ([A-Z]{1,2})?(\d+)\b", raw):
        prefix, first, last = match.group(1), int(match.group(2)), int(match.group(4))
        for number in range(first, last + 1):
            for candidate in sorted(known):
                if re.fullmatch(rf"{prefix}{number}[a-z]?", candidate) and candidate not in found:
                    found.append(candidate)
    for token in re.findall(r"\b[A-Z]{1,2}\d+[a-z]?\b", raw):
        if token in known and token not in found:
            found.append(token)
    return found, raw


def registry() -> dict:
    path = SPEC / "registry.toml"
    if not path.exists():
        return {}
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    for extra in sorted((SPEC / "registry.d").glob("*.toml")):
        for kind, entries in tomllib.loads(extra.read_text(encoding="utf-8")).items():
            data.setdefault(kind, {}).update(entries or {})
    return data


def write_registry_block(packages: list[dict]) -> None:
    path = SPEC / "registry.toml"
    lines = [REG_START, "# Written by rewrite/tools/gen_packages.py from section 9 of the blueprint. Do not edit by hand."]
    for package in packages:
        summary = package["title"].replace("\\", "\\\\").replace('"', '\\"')
        lines.append(f'[package.{package["id"]}]\nsummary = "{summary}"')
    lines.append(REG_END)
    block = "\n".join(lines) + "\n"
    if path.exists():
        text = path.read_text(encoding="utf-8")
        if REG_START in text and REG_END in text:
            before = text[: text.index(REG_START)]
            after = text[text.index(REG_END) + len(REG_END) :].lstrip("\n")
            text = before + block + ("\n" + after if after else "")
        else:
            text = block + "\n" + text
    else:
        text = "# The registry of every thing in the 1.0 design. See README.md.\n\n" + block
    path.write_text(text, encoding="utf-8")


def added_packages(planned: set[str]) -> list[dict]:
    """Packages added after the blueprint's build plan was written.

    One is registered in a rewrite/spec/registry.d/ file as [package.<ID>] with `summary`, `delivers`,
    `depends` (text naming package ids, or "nothing") and `source` (the decision that added it, for
    example "decisions.md#D31"). Its brief is generated the same way as a planned package's.
    """
    found: list[dict] = []
    for extra in sorted((SPEC / "registry.d").glob("*.toml")):
        entries = tomllib.loads(extra.read_text(encoding="utf-8")).get("package", {})
        for ident, entry in entries.items():
            if ident in planned:
                raise SystemExit(f"registry.d/{extra.name}: package {ident} is already in the blueprint's build plan")
            if not PACKAGE_ID.match(ident):
                raise SystemExit(f"registry.d/{extra.name}: {ident} is not a valid package id")
            missing = [key for key in ("summary", "delivers", "depends", "source") if not entry.get(key)]
            if missing:
                raise SystemExit(f"registry.d/{extra.name}: package {ident} needs {', '.join(missing)}")
            found.append({"id": ident, "title": entry["summary"], "delivers": entry["delivers"],
                          "depends_raw": entry["depends"], "source": entry["source"]})
    return found


def main() -> int:
    packages = parse_plan()
    SPEC.mkdir(parents=True, exist_ok=True)
    write_registry_block(packages)
    packages += added_packages({p["id"] for p in packages})
    known = {p["id"] for p in packages}
    reg = registry()
    implements: dict[str, list[tuple[str, str, str]]] = {}
    for kind, entries in reg.items():
        if kind == "package" or not isinstance(entries, dict):
            continue
        for ident, entry in entries.items():
            package = entry.get("package") if isinstance(entry, dict) else None
            if package and package != "none":
                implements.setdefault(package, []).append((kind, ident, entry.get("summary", "")))
    out = SPEC / "packages"
    out.mkdir(parents=True, exist_ok=True)
    for package in packages:
        ident = package["id"]
        depends, raw = expand_depends(package["depends_raw"], known - {ident})
        body = [START, "## Delivers", "", package["delivers"] or f"The `{package['title']}` workflow script with its scenario tests.", ""]
        body += ["## Depends on", ""]
        body += [f"- [[package:{d}]]" for d in depends] or ["- nothing"]
        body += ["", f"As written in the build plan: {raw}", "", "## Implements", ""]
        mine = sorted(implements.get(ident, []))
        body += [f"- [[{kind}:{thing}]]: {summary}" for kind, thing, summary in mine] or ["- nothing registered yet"]
        body += ["", "## Read first", "", "- [[doc:conventions]]", "- every file listed under Implements, and the files those link to", END]
        path = out / f"{ident}.md"
        notes = "\n## Notes\n"
        if path.exists():
            old = path.read_text(encoding="utf-8")
            if END in old:
                notes = old[old.index(END) + len(END) :]
        front = "\n".join(
            [
                "---",
                "kind: package",
                f"id: {ident}",
                f"depends_on: [{', '.join(depends)}]",
                f"sources: [{package.get('source') or 'BLUEPRINT.md#' + package['section']}]",
                "---",
                "",
                f"# {ident}: {package['title']}",
                "",
            ]
        )
        path.write_text(front + "\n".join(body) + notes, encoding="utf-8")
    unknown = sorted(p for p in implements if p not in known)
    print(f"{len(packages)} package briefs written to {out.relative_to(REWRITE.parent)}")
    empty = [p["id"] for p in packages if not implements.get(p["id"])]
    if empty:
        print(f"packages with nothing registered yet: {', '.join(empty)}")
    if unknown:
        print(f"registry names packages that are not in the build plan: {', '.join(unknown)}")
    return 1 if unknown else 0


if __name__ == "__main__":
    sys.exit(main())
