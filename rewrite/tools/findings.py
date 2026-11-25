#!/usr/bin/env python3
"""The findings ledger: every open problem with the design, in one place, each with an outcome.

`findings.py build` reads the raw results saved under rewrite/findings/sources/ and the outcome files
under rewrite/findings/outcomes/, and writes:
- rewrite/findings/ledger.json: one entry per finding, with the spec files it concerns and its status;
- rewrite/findings/by-package/<ID>.md: the open findings that concern the files a build package implements.

A finding is closed only by an entry in an outcome file, written when a fix has been made and checked.
Standard library only.
"""
from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path

REWRITE = Path(__file__).resolve().parent.parent
SPEC = REWRITE / "spec"
FINDINGS = REWRITE / "findings"
DOC_NAMES = ["BLUEPRINT.md", "pipeline.md", "state.md", "agents-skills.md", "hooks.md", "packaging.md"]
DOC_RE = re.compile(r"(BLUEPRINT\.md|pipeline\.md|state\.md|agents-skills\.md|hooks\.md|packaging\.md)", re.I)
NUM_RE = re.compile(r"(?<![\w.])(\d{1,2}(?:\.\d{1,2}){0,3})(?![\w])")
SPEC_PATH_RE = re.compile(
    r"(?:rewrite/spec/)?((?:topics|commands|records|schemas|rules|agents|skills|workflows|artifacts|tables|config|packaging)"
    r"/[A-Za-z0-9_.-]+\.(?:md|toml|json))"
)
ACTIVE = {"open", "not-fixed", "left-for-builder"}  # statuses a builder must still act on
SEVERITY_ORDER = {"blocker": 0, "major": 1, "minor": 2}


def section_ids(text: str, known: set[str]) -> list[str]:
    """Section ids named in free text such as 'rewrite/design/pipeline.md 2.9.4 row, 2.6; BLUEPRINT.md 4.9'."""
    found: list[str] = []
    pieces = DOC_RE.split(text)
    for index in range(1, len(pieces), 2):
        doc = next(d for d in DOC_NAMES if d.lower() == pieces[index].lower())
        tail = pieces[index + 1] if index + 1 < len(pieces) else ""
        for number in NUM_RE.findall(tail.replace("#", " ")):
            candidate = f"{doc}#{number}"
            while candidate not in known and "." in candidate.split("#")[1]:
                candidate = candidate.rsplit(".", 1)[0]
            if candidate in known and candidate not in found:
                found.append(candidate)
    return found


def load_sources(known: set[str]) -> list[dict]:
    out: list[dict] = []
    src = FINDINGS / "sources"

    def add(ident, origin, severity, where_text, problem, proposed="", slice_path=None, extra=None):
        out.append(
            {
                "id": ident,
                "origin": origin,
                "severity": severity,
                "on_slice_path": slice_path,
                "where": where_text,
                "sections": section_ids(where_text, known),
                "spec_files": sorted({m for m in SPEC_PATH_RE.findall(where_text) if (SPEC / m).exists()}),
                "problem": problem,
                "proposed_resolution": proposed,
                **(extra or {}),
            }
        )

    p = src / "pass3.json"
    if p.exists():
        for f in json.loads(p.read_text())["findings"]:
            add(f["id"], "review pass 3", f["severity"], "; ".join(f["documents"]), f["problem"], f["proposed_resolution"], f["on_slice_path"])
    p = src / "closeout-fix.json"
    if p.exists():
        for f in json.loads(p.read_text())["final"]["remaining"]:
            add(f"CL-{f['id']}", "close-out of pass 2", f["severity"], f["where"], f["problem"], f.get("proposed_resolution", ""), False)
    p = src / "slice-layer-1.json"
    if p.exists():
        order = sorted(json.loads(p.read_text())["design_findings"], key=lambda x: (SEVERITY_ORDER[x["severity"]], x["source"]))
        for n, f in enumerate(order, 1):
            add(f"SL1-{n:02d}", f"slice layer 1 ({f['source']})", f["severity"], f["where"], f["problem"], "", True, {"what_was_built": f.get("what_i_did", "")})
    p = src / "slice-amend-result.json"
    if p.exists():
        for f in json.loads(p.read_text())["verify"]["remaining"]:
            if f["id"].startswith("VSA-"):
                add(f["id"], "verification of the slice amendment", f["severity"], f["where"], f["problem"], f.get("proposed_resolution", ""), None)
    # Findings reported while building: any sources/extra-*.json holding a list of
    # {id, origin, severity, where, problem, proposed_resolution?, on_slice_path?}.
    for extra in sorted(src.glob("extra-*.json")):
        for f in json.loads(extra.read_text()):
            add(f["id"], f.get("origin", extra.stem), f["severity"], f.get("where", ""), f["problem"],
                f.get("proposed_resolution", ""), f.get("on_slice_path"))
    p = src / "restructure-audit-2.json"
    if p.exists():
        for f in json.loads(p.read_text())["findings"]:
            add(f"AU-{f['id']}", f"restructure audit 2 ({f['kind']})", f["severity"], f["where"], f["problem"], f["proposed_resolution"], None)
    return out


def main() -> int:
    manifest = json.loads((REWRITE / "history" / "section-manifest.json").read_text())
    known = {s["id"] for s in manifest}
    index_file = SPEC / "source-index.json"
    index = json.loads(index_file.read_text()) if index_file.exists() else {}
    findings = load_sources(known)

    # Outcome files apply in the order they were recorded, so a later check that reopens a finding, or a
    # later fix that closes it, wins. The order is kept in outcomes/ORDER; a file not yet listed there is
    # appended, oldest first.
    outcome_dir = FINDINGS / "outcomes"
    order_file = outcome_dir / "ORDER"
    order = [line.strip() for line in order_file.read_text().split("\n") if line.strip()] if order_file.exists() else []
    present = {path.name: path for path in outcome_dir.glob("*.json")}
    order = [name for name in order if name in present]
    for path in sorted(present.values(), key=lambda q: (q.stat().st_mtime, q.name)):
        if path.name not in order:
            order.append(path.name)
    order_file.write_text("\n".join(order) + "\n")
    outcomes: dict[str, dict] = {}
    for name in order:
        for entry in json.loads(present[name].read_text()):
            outcomes[entry["id"]] = {**entry, "recorded_in": name}

    # A triage file names, for each finding, the few spec files where its fix belongs and the packages
    # that must act on it. Without one, the files are guessed from the sections the finding cites,
    # which is far too broad for a section that many files cite.
    triage: dict[str, dict] = {}
    for path in sorted((FINDINGS / "triage").glob("*.json")):
        for entry in json.loads(path.read_text()):
            triage[entry["id"]] = entry

    for f in findings:
        files = set(f["spec_files"])
        chosen = triage.get(f["id"])
        if chosen:
            files = set(chosen.get("files", []))
            f["packages"] = sorted(set(chosen.get("packages", [])))
            if f["on_slice_path"] is None:
                f["on_slice_path"] = bool(chosen.get("on_slice_path"))
            f["triaged"] = True
        else:
            for section in f["sections"]:
                files.update(index.get(section, []))
            f["triaged"] = False
        f["spec_files"] = sorted(files)
        outcome = outcomes.get(f["id"])
        f["status"] = outcome["outcome"] if outcome else "open"
        if outcome:
            f["outcome"] = outcome

    registry = tomllib.loads((SPEC / "registry.toml").read_text())
    for extra in sorted((SPEC / "registry.d").glob("*.toml")):
        for kind, entries in tomllib.loads(extra.read_text()).items():
            registry.setdefault(kind, {}).update(entries or {})
    folders = {"doc": "", "topic": "topics", "command": "commands", "record": "records", "schema": "schemas", "rule": "rules",
               "agent": "agents", "skill": "skills", "workflow": "workflows", "artifact": "artifacts", "table": "tables",
               "config": "config", "packaging": "packaging", "package": "packages"}
    suffix = {"schema": ".schema.json", "table": ".toml"}
    package_files: dict[str, set[str]] = {}
    for kind, entries in registry.items():
        for ident, entry in entries.items():
            package = entry.get("package") if isinstance(entry, dict) else None
            if package and package != "none":
                folder = folders[kind]
                name = f"{ident}{suffix.get(kind, '.md')}"
                package_files.setdefault(package, set()).add(f"{folder}/{name}" if folder else name)

    FINDINGS.mkdir(exist_ok=True)
    (FINDINGS / "ledger.json").write_text(json.dumps(findings, indent=1) + "\n")
    out = FINDINGS / "by-package"
    out.mkdir(exist_ok=True)
    for old in out.glob("*.md"):
        old.unlink()
    for package, files in sorted(package_files.items()):
        mine = [
            f
            for f in findings
            if f["status"] in ACTIVE and (package in f["packages"] if f.get("triaged") else files & set(f["spec_files"]))
        ]
        if not mine:
            continue
        mine.sort(key=lambda f: (SEVERITY_ORDER[f["severity"]], f["id"]))
        lines = [f"# Open findings that concern package {package}", "",
                 f"{len(mine)} to act on. Each names files this package implements. Generated by rewrite/tools/findings.py; do not edit.", "",
                 "Status: open means nobody has dealt with it; not-fixed means a fix was tried and a check found it incomplete;",
                 "left-for-builder means the design was settled and the code must now follow it.", ""]
        for f in mine:
            lines += [f"## {f['id']} ({f['severity']}; {f['origin']}; {f['status']})", ""]
            if f.get("outcome"):
                lines += ["Earlier outcome: " + f["outcome"].get("what_was_done", "").strip(), ""]
            lines += ["Files: " + ", ".join(f["spec_files"] if f.get("triaged") else sorted(files & set(f["spec_files"]))), "",
                      "Where it was found: " + f["where"][:600], "",
                      f["problem"].strip(), ""]
            if f.get("proposed_resolution"):
                lines += ["Proposed resolution: " + f["proposed_resolution"].strip(), ""]
            if f.get("what_was_built"):
                lines += ["What layer 1 built: " + f["what_was_built"].strip(), ""]
        (out / f"{package}.md").write_text("\n".join(lines))

    total = len(findings)
    by_status: dict[str, int] = {}
    for f in findings:
        by_status[f["status"]] = by_status.get(f["status"], 0) + 1
    unmapped = [f["id"] for f in findings if not f["spec_files"]]
    open_now = [f for f in findings if f["status"] in ACTIVE]
    print(f"{total} findings: " + ", ".join(f"{n} {s}" for s, n in sorted(by_status.items())))
    print("open by severity: " + ", ".join(f"{sum(1 for f in open_now if f['severity'] == s)} {s}" for s in SEVERITY_ORDER))
    print(f"open on the slice path: {sum(1 for f in open_now if f['on_slice_path'])}")
    print(f"triaged: {sum(1 for f in findings if f.get('triaged'))} of {total}")
    print(f"mapped to no spec file: {len(unmapped)}" + (f" ({', '.join(unmapped[:12])}{' ...' if len(unmapped) > 12 else ''})" if unmapped else ""))
    print(f"by-package lists written: {len(list(out.glob('*.md')))}")
    return 0


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "build":
        sys.exit(main())
    print(__doc__)
    sys.exit(2)
