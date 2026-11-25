#!/usr/bin/env python3
"""Section manifest for the original design documents.

`sections.py manifest` records every section of the six original documents (id, title, lines, bytes).
`sections.py coverage` reports which sections no spec file cites and which are retired, so that the
restructure into rewrite/spec/ can be shown to have lost nothing.

Standard library only. Run from the repository root.
"""
from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path

REWRITE = Path(__file__).resolve().parent.parent
DOCS = {
    "BLUEPRINT.md": [REWRITE / "BLUEPRINT.md", REWRITE / "history" / "design-v1" / "BLUEPRINT.md"],
    "pipeline.md": [REWRITE / "design" / "pipeline.md", REWRITE / "history" / "design-v1" / "pipeline.md"],
    "state.md": [REWRITE / "design" / "state.md", REWRITE / "history" / "design-v1" / "state.md"],
    "agents-skills.md": [REWRITE / "design" / "agents-skills.md", REWRITE / "history" / "design-v1" / "agents-skills.md"],
    "hooks.md": [REWRITE / "design" / "hooks.md", REWRITE / "history" / "design-v1" / "hooks.md"],
    "packaging.md": [REWRITE / "design" / "packaging.md", REWRITE / "history" / "design-v1" / "packaging.md"],
}
MANIFEST = REWRITE / "history" / "section-manifest.json"
SPEC = REWRITE / "spec"
HEADING = re.compile(r"^(#{2,4}) +(.*?)\s*$")
NUMBER = re.compile(r"^(\d+(?:\.\d+)*)\.?\s")
SMALL = 200  # sections with less own text than this need no citation


def locate(name: str) -> Path:
    for candidate in DOCS[name]:
        if candidate.exists():
            return candidate
    raise SystemExit(f"cannot find {name}")


def slug(title: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", title.lower()).strip("-")[:48]


def sections_of(name: str) -> list[dict]:
    lines = locate(name).read_text(encoding="utf-8").split("\n")
    out: list[dict] = []
    seen: dict[str, int] = {}
    fenced = False
    for index, line in enumerate(lines):
        if line.startswith("```"):
            fenced = not fenced
            continue
        if fenced:
            continue
        match = HEADING.match(line)
        if not match:
            continue
        title = match.group(2)
        number = NUMBER.match(title)
        key = number.group(1) if number else slug(title)
        seen[key] = seen.get(key, 0) + 1
        if seen[key] > 1:
            key = f"{key}-{seen[key]}"
        out.append({"id": f"{name}#{key}", "doc": name, "level": len(match.group(1)), "title": title, "start": index + 1})
    for position, section in enumerate(out):
        end = out[position + 1]["start"] - 1 if position + 1 < len(out) else len(lines)
        section["end"] = end
        body = "\n".join(lines[section["start"] : end])
        section["bytes"] = len(body.strip().encode("utf-8"))
    return out


def manifest() -> int:
    everything: list[dict] = []
    for name in DOCS:
        everything.extend(sections_of(name))
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(json.dumps(everything, indent=1) + "\n", encoding="utf-8")
    total = sum(s["bytes"] for s in everything)
    print(f"{len(everything)} sections, {total // 1024} KB of section text, written to {MANIFEST.relative_to(REWRITE.parent)}")
    for name in DOCS:
        mine = [s for s in everything if s["doc"] == name]
        print(f"  {name:<18} {len(mine):>3} sections")
    return 0


def frontmatter_sources(path: Path) -> list[str]:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        return []
    block = text[4:].split("\n---", 1)[0]
    found: list[str] = []
    capture = False
    for line in block.split("\n"):
        if line.startswith("sources:"):
            rest = line[len("sources:") :].strip()
            if rest.startswith("["):
                found.extend(x.strip().strip("\"'") for x in rest.strip("[]").split(",") if x.strip())
                capture = False
            else:
                capture = True
            continue
        if capture:
            if line.startswith("  - ") or line.startswith("- "):
                found.append(line.split("- ", 1)[1].strip().strip("\"'"))
            elif line.strip():
                capture = False
    return found


def coverage() -> int:
    if not MANIFEST.exists():
        raise SystemExit("run `sections.py manifest` first")
    sections = json.loads(MANIFEST.read_text(encoding="utf-8"))
    ids = {s["id"] for s in sections}
    # A decision adopted after the original documents were written is a source in its own right:
    # `decisions.md#D31` is accepted when rewrite/spec/decisions.md has the heading `### D31`.
    # The decisions are kept in decisions.md (D1 to D30) and decisions-later.md (D31 onwards); the source is
    # written `decisions.md#D<n>` for either.
    decisions_text = "".join(
        (SPEC / name).read_text(encoding="utf-8") for name in ("decisions.md", "decisions-later.md") if (SPEC / name).exists()
    )
    later_decisions = {f"decisions.md#{line[4:].strip()}" for line in decisions_text.split("\n") if line.startswith("### D")}
    cited: dict[str, list[str]] = {}
    unknown: list[tuple[str, str]] = []
    for path in sorted(SPEC.rglob("*.md")):
        for source in frontmatter_sources(path):
            if source in ids:
                cited.setdefault(source, []).append(str(path.relative_to(SPEC)))
            elif source not in later_decisions:
                unknown.append((str(path.relative_to(SPEC)), source))
    retired: dict[str, str] = {}
    retired_file = SPEC / "retired.toml"
    if retired_file.exists():
        data = tomllib.loads(retired_file.read_text(encoding="utf-8"))
        for entry in data.get("retired", []):
            retired[entry["id"]] = entry.get("reason", "")
    uncovered = [s for s in sections if s["bytes"] >= SMALL and s["id"] not in cited and s["id"] not in retired]
    bad_retired = [i for i in retired if i not in ids]
    covered_bytes = sum(s["bytes"] for s in sections if s["id"] in cited)
    retired_bytes = sum(s["bytes"] for s in sections if s["id"] in retired and s["id"] not in cited)
    total = sum(s["bytes"] for s in sections)
    print(f"sections: {len(sections)}; cited by a spec file: {len(cited)}; retired: {len(retired)}; uncovered: {len(uncovered)}")
    print(f"text: {total // 1024} KB total; {covered_bytes // 1024} KB cited; {retired_bytes // 1024} KB retired")
    for path, source in unknown[:40]:
        print(f"UNKNOWN SOURCE  {path}: {source}")
    for item in bad_retired[:40]:
        print(f"UNKNOWN RETIRED {item}")
    for s in uncovered:
        print(f"UNCOVERED  {s['id']}  ({s['bytes']} bytes)  {s['title'][:70]}")
    if "--write-index" in sys.argv:
        index = {s["id"]: cited.get(s["id"], []) for s in sections}
        (SPEC / "source-index.json").write_text(json.dumps(index, indent=1) + "\n", encoding="utf-8")
        print(f"wrote {(SPEC / 'source-index.json').relative_to(REWRITE.parent)}")
    return 1 if (uncovered or unknown or bad_retired) else 0


if __name__ == "__main__":
    command = sys.argv[1] if len(sys.argv) > 1 else ""
    if command == "manifest":
        sys.exit(manifest())
    if command == "coverage":
        sys.exit(coverage())
    print(__doc__)
    sys.exit(2)
