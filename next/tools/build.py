#!/usr/bin/env python3
"""Generate the committed copies that cannot drift from their sources.

Generates, in this order (packaging.md 2.5, as corrected by P3BLD-21):

1. the ``TIER``/``ROLE``/``OVERRIDE`` tables inside ``workflow-lib/prelude.js``, from
   ``tests/lint/roster.toml``;
2. the schema literals inside ``workflow-lib/prelude.js``, from ``<plugin>/schemas/*.schema.json``
   (``$ref`` is inlined, so scripts compose shapes without refs);
3. the stamped prelude region of every workflow file (``<plugin>/workflows/*.js`` and the harness's
   toy workflows under ``tests/workflows/toys/*.js``);
4. the stamped shared block in every agent file (``<plugin>/agents/*.md``) — the spike-runner and
   clerk variants receive their own blocks (agents-skills.md 2.7) — and the stamped severity table
   plus finding-key rule in every reviewer agent file (an agent the roster's tier makes a reviewer);
5. ``<plugin>/hooks/hooks.json``, from the rule registry (``vsdd/hooks/registry.py``, B4b) — a rule
   cannot be registered and unwired, which is what happened to the old pre-compaction hooks.

``--check`` changes nothing and exits 1, naming each problem and each drifting file, when any
generated copy differs or a marker pair is not exactly one opening followed by one closing in place.

Standard library only. Run from ``next/``: ``uv run --no-sync python tools/build.py [--check]``.
"""

from __future__ import annotations

import json
import re
import sys
import tomllib
from pathlib import Path

NEXT = Path(__file__).resolve().parent.parent
PLUGIN = NEXT / "plugins" / "vsdd-factory"

sys.dont_write_bytecode = True
sys.path.insert(0, str(PLUGIN / "py"))

PRELUDE_SRC = NEXT / "workflow-lib" / "prelude.js"
ROSTER = NEXT / "tests" / "lint" / "roster.toml"

WORKFLOW_DIRS = [PLUGIN / "workflows", NEXT / "tests" / "workflows" / "toys"]
AGENTS_DIR = PLUGIN / "agents"

TABLE_MARKERS = ("// >>> generated:tables", "// <<< generated:tables")
SCHEMA_MARKERS = ("// >>> generated:schemas", "// <<< generated:schemas")
PRELUDE_MARKERS = ("// >>> stamped:prelude", "// <<< stamped:prelude")
SHARED_MARKERS = ("<!-- >>> stamped:shared -->", "<!-- <<< stamped:shared -->")
SEVERITY_MARKERS = ("<!-- >>> stamped:severity -->", "<!-- <<< stamped:severity -->")
KEYRULE_MARKERS = ("<!-- >>> stamped:keyrule -->", "<!-- <<< stamped:keyrule -->")

FINDING_KEY_RULE = (
    "The finding key is `<category>:<path>:<symbol or clause>`: stable, so the same defect carries "
    "the same key in every pass."
)


def _marker_problems(text: str, markers: tuple[str, str], label: str) -> list[str]:
    """Exactly one opening and one closing marker, opening before closing."""
    opening, closing = markers
    problems: list[str] = []
    openings = text.count(opening)
    closings = text.count(closing)
    if openings != 1:
        problems.append(f"{label}: {openings} opening markers {opening!r}, exactly 1 expected")
    if closings != 1:
        problems.append(f"{label}: {closings} closing markers {closing!r}, exactly 1 expected")
    if openings == 1 and closings == 1:
        if text.find(opening) > text.find(closing):
            problems.append(f"{label}: the opening marker comes after the closing marker")
    return problems


def _replace_between(text: str, markers: tuple[str, str], inner: str) -> str:
    """The text with the region between the markers replaced by ``inner``.

    Validates the pair first: a missing or duplicated marker is a named
    problem, never a silent rewrite that keeps a tail of the file.
    """
    problems = _marker_problems(text, markers, "region")
    if problems:
        raise MarkerProblem("; ".join(problems))
    opening, closing = markers
    start = text.find(opening) + len(opening)
    end = text.find(closing)
    return text[:start] + "\n" + inner.rstrip("\n") + "\n" + text[end:]


class MarkerProblem(ValueError):
    """A marker pair is missing, duplicated or out of order; nothing is written."""


def _lines(label: str, obj) -> str:
    return json.dumps(obj, indent=2, ensure_ascii=False)


def _strip_schema_node(node, by_id: dict, *, in_properties: bool = False):
    """Remove $schema/$id/description decoration and inline $ref siblings.

    Annotation keywords are stripped only where they are keywords: a property
    literally named ``description`` inside a ``properties`` mapping is a field
    of the data, not an annotation, and survives.
    """
    if isinstance(node, list):
        return [_strip_schema_node(x, by_id) for x in node]
    if not isinstance(node, dict):
        return node
    if "$ref" in node:
        ref = node["$ref"].split("/")[-1]
        if ref not in by_id:
            raise ValueError(f"unresolvable $ref {node['$ref']!r}")
        if set(node) - {"$ref"}:
            raise ValueError(f"$ref with siblings is not supported: {sorted(node)}")
        return _strip_schema_node(by_id[ref], by_id)
    out = {}
    for key, value in node.items():
        if key == "properties" and isinstance(value, dict):
            out[key] = {
                name: _strip_schema_node(sub, by_id, in_properties=True)
                for name, sub in value.items()
            }
            continue
        if in_properties and key in ("description", "$id", "$schema"):
            # A property named like an annotation is data; only an annotation
            # sitting beside other schema keys is decoration.
            rest = {k: v for k, v in node.items() if k != key}
            if set(rest) & {"type", "properties", "required", "enum", "items"}:
                out[key] = _strip_schema_node(value, by_id, in_properties=in_properties)
                continue
        if key in ("$schema", "$id", "description") and not in_properties:
            continue
        out[key] = _strip_schema_node(value, by_id, in_properties=in_properties)
    return out


# Generated block 1: the tier, role and override tables from the roster.


def _generate_tables() -> str:
    roster = tomllib.loads(ROSTER.read_text())
    tiers = {name: {"model": tier["model"], "effort": tier["effort"]} for name, tier in roster["tiers"].items()}
    roles = dict(roster["agents"])
    overrides = dict(roster.get("overrides", {}))
    out = (
        f"const TIER = {_lines('TIER', tiers)};\n"
        f"const ROLE = {_lines('ROLE', roles)};\n"
        "// The only steps that run off their agent's tier (decision D5). step() consults it by \"<workflow>/<stage>\";\n"
        "// a script cannot mark one of its own, and lint rejects any override the roster does not list.\n"
        f"const OVERRIDE = {_lines('OVERRIDE', overrides)};"
    )
    return out


# Generated block 2: the schema literals from <plugin>/schemas/*.json.

LITERAL_NAMES = {
    "finding.schema.json": "S_FINDING",
    "gate-result.schema.json": "S_GATE",
    "pending-output.schema.json": "S_PENDING_OUTPUT",
    "review-data.schema.json": "S_REVIEW_DATA",
    "claim.schema.json": "S_CLAIM",
    "fix-data.schema.json": "S_FIX_DATA",
    "ruling-data.schema.json": "S_RULING_DATA",
    "holdout-data.schema.json": "S_HOLDOUT_DATA",
    "intake-data.schema.json": "S_INTAKE_DATA",
    "implement-data.schema.json": "S_IMPLEMENT_DATA",
    "story-unit.schema.json": "S_STORY_UNIT",
    "story-draft-data.schema.json": "S_STORY_DRAFT_DATA",
    "spike-request.schema.json": "S_SPIKE_REQUEST",
    "spike-finding.schema.json": "S_SPIKE_FINDING",
    "step-result.schema.json": "S_STEP_RESULT",
    "verdict.schema.json": "S_VERDICT",
    "workflow-result.schema.json": "S_WORKFLOW_RESULT",
    "survey.schema.json": "S_SURVEY",
    "checkpoint-result.schema.json": "S_CLOSE",
    "delivery-data.schema.json": "S_DELIVERY",
    "wave-gate-data.schema.json": "S_WAVE_GATE",
}


def _generate_schemas() -> str:
    by_id = _shipped_schemas()
    parts = []
    for filename, name in LITERAL_NAMES.items():
        if filename not in by_id:
            continue  # named by _mapping_problems; never crash the build on a mapping typo
        schema = _strip_schema_node(by_id[filename], by_id)
        parts.append(f"const {name} = {_lines(name, schema)};")
    return "\n".join(parts)


def _shipped_schemas() -> dict[str, dict]:
    return {p.name: json.loads(p.read_text()) for p in sorted((PLUGIN / "schemas").glob("*.schema.json"))}


# Shipped schemas that are not prelude literals: the event log line is the
# Python writer's schema (py/vsdd/events.py loads the file), not a workflow's,
# and the caller marker is the runner's own file (HD13; record caller-marker).
NON_PRELUDE_SCHEMAS = {
    "event.schema.json",
    "caller-marker.schema.json",
    "run-record.schema.json",
    "step-record.schema.json",
    "evidence-record.schema.json",
    "closure-record.schema.json",
    "cycle-record.schema.json",
    "decision-record.schema.json",
    "lock.schema.json",
}

HAND_LITERAL_RE = re.compile(r"^\s*const\s+(S_[A-Z0-9_]+)\s*=", re.M)


def _hand_literal_problems(path: Path, text: str, regions: list[tuple[str, str]]) -> list[str]:
    """Every hand-written `const S_* =` outside the generated or stamped regions.

    The build generates every S_* literal from <plugin>/schemas/*.schema.json;
    one written by hand in prelude.js's own code or in a workflow body is a
    second source that --check cannot see (result-schemas.md: no S_* literal is
    written by hand).
    """
    outside = text
    for opening, closing in regions:
        start = outside.find(opening)
        end = outside.find(closing)
        if start < 0 or end < 0:
            continue
        outside = outside[:start] + outside[end + len(closing):]
    return [
        f"{path}: hand-written schema literal {name} "
        "(the build alone writes S_* constants, into the generated block)"
        for name in HAND_LITERAL_RE.findall(outside)
    ]


def _mapping_problems() -> list[str]:
    """LITERAL_NAMES must cover exactly the shipped schema files it does not map by name."""
    shipped = set(_shipped_schemas())
    mapped = set(LITERAL_NAMES) | NON_PRELUDE_SCHEMAS
    problems: list[str] = []
    if mapped - shipped:
        problems.append(
            "LITERAL_NAMES names schemas that are not shipped: " + ", ".join(sorted(mapped - shipped))
        )
    if shipped - mapped:
        problems.append(
            "shipped schemas no literal mapping covers (add to LITERAL_NAMES or NON_PRELUDE_SCHEMAS): "
            + ", ".join(sorted(shipped - mapped))
        )
    return problems


# Generated block 3: the stamped prelude region of every workflow.


def _workflow_files() -> list[Path]:
    files: list[Path] = []
    for directory in WORKFLOW_DIRS:
        files.extend(sorted(directory.glob("*.js")))
    return files


def meta_ok(text: str, path: Path) -> list[str]:
    """The layout rule, kept simple so no JavaScript parser is needed (packaging.md 2.5)."""
    problems = []
    lines = text.splitlines()
    if not lines or lines[0] != "export const meta = {":
        problems.append(f"{path}: line 1 must be exactly 'export const meta = {{'")
        return problems
    try:
        end = lines.index("}", 1)
    except ValueError:
        problems.append(f"{path}: the meta object never closes on a lone '}}' line")
        return problems
    rest = [line for line in lines[end + 1 :] if line.strip()]
    if not rest or not rest[0].startswith(PRELUDE_MARKERS[0]):
        problems.append(f"{path}: the opening prelude marker must follow the meta object")
    return problems


def generate_workflow_body(path: Path, prelude: str) -> str:
    text = path.read_text()
    if PRELUDE_MARKERS[0] in text:
        return _replace_between(text, PRELUDE_MARKERS, prelude)
    return insert_after_meta(text, prelude)


def insert_after_meta(text: str, prelude: str) -> str:
    """Insert the stamped region after the meta object.

    The region is produced by the same `_replace_between` that later replaces
    it, so insert and replace are byte-identical and the next ``--check`` is
    clean. The provenance note rides on the closing marker line as a comment.
    """
    lines = text.splitlines()
    end = lines.index("}", 1)
    preamble = "\n".join(lines[: end + 1]) + "\n"
    rest = "\n".join(lines[end + 1 :])
    synthetic = (
        PRELUDE_MARKERS[0]
        + "\n"
        + PRELUDE_MARKERS[1]
        + " // generated from workflow-lib/prelude.js by tools/build.py; do not edit between the markers\n"
        + rest
    )
    replaced = _replace_between(synthetic, PRELUDE_MARKERS, prelude)
    return preamble + replaced


# Generated block 4: the shared block (per variant) and the reviewer stamps in agent files.


def _stamp_shared(text: str, shared: str) -> str:
    if SHARED_MARKERS[0] in text:
        return _replace_between(text, SHARED_MARKERS, shared)
    return text.rstrip("\n") + "\n\n" + SHARED_MARKERS[0] + "\n" + shared.rstrip("\n") + "\n" + SHARED_MARKERS[1] + "\n"


def _stamp_region(text: str, markers: tuple[str, str], inner: str) -> str:
    stamped = f"{markers[0]}\n{inner.rstrip(chr(10))}\n{markers[1]}"
    if markers[0] in text:
        return _replace_between(text, markers, inner)
    return text.rstrip("\n") + "\n\n" + stamped + "\n"


def _frontmatter_field(text: str, field: str) -> str:
    """One frontmatter value; the scan stops at the closing fence."""
    inside = False
    for line in text.splitlines():
        if line.strip() == "---":
            if inside:
                break
            inside = True
            continue
        if inside and line.strip().startswith(f"{field}:"):
            return line.split(":", 1)[1].strip()
    return ""


def _reviewer_variant(name: str) -> str:
    """The shared-block variant an agent receives (agents-skills.md 2.7).

    The clerk carries a two-line block of its own; the spike-runner omits the
    dispatch bullet, because a spike cannot start a spike; every other agent
    gets the full block.
    """
    if name == "clerk":
        return (
            "## Ground rules\n"
            "- The spec is the authority. If it is wrong or silent, say so in your result. Do not work around it.\n"
            "- Run exactly the command you were given and return its output. Do not interpret, repair or summarise it."
        )
    if name == "spike-runner":
        return (
            "## Ground rules\n"
            "- The spec is the authority. If it is wrong or silent, say so in your result. Do not work around it.\n"
            "- Report only what you checked. Name the command you ran or the file you read. If a check did not run, say so.\n"
            "- Done means on disk. Write your work, and commit code, before you return. Your result points to it.\n"
            "- Stay in your role. If a fix belongs to another role, describe it in your result and leave it.\n"
            "- Use the paths you were given, as given. Do not build a path into the state directory yourself.\n"
            "- If a hook blocks a tool call, read its reason and follow it. Do not reach the same effect another way.\n"
            "\n"
            "## When you cannot proceed\n"
            "- A missing input, or a problem you could not fix in two attempts: stop and return blocked, with what you tried."
        )
    return (NEXT / "workflow-lib" / "agent-shared.md").read_text().rstrip("\n")


def is_reviewer(name: str, roster: dict, text: str = "") -> bool:
    """Whether an agent file takes the reviewer stamps (severity table, key rule).

    The one decision the build and the roster lint both call. A rostered agent
    is a reviewer when its tier is, which the model comparison cannot express
    ('reviewer' is a tier name, not a model); the model is the fallback only
    for an agent no roster row lists.
    """
    tier = roster.get("agents", {}).get(name)
    if tier is not None:
        return tier == "reviewer"
    reviewer_model = roster["tiers"]["reviewer"]["model"]
    return bool(text) and _frontmatter_field(text, "model") == reviewer_model


def _display(path: Path) -> str:
    """The path for a report: under next/ relative, else the full path."""
    try:
        return str(path.relative_to(NEXT))
    except ValueError:
        return str(path)


def _stamp_agents(problems: list[str], drifted: list[str], check_only: bool) -> None:
    roster = tomllib.loads(ROSTER.read_text())
    agents = roster.get("agents", {})
    severity = (NEXT / "workflow-lib" / "severity.md").read_text()
    keyrule_src = NEXT / "workflow-lib" / "finding-key.md"
    keyrule = keyrule_src.read_text().rstrip("\n") if keyrule_src.is_file() else FINDING_KEY_RULE
    agent_files = sorted(AGENTS_DIR.glob("*.md")) if AGENTS_DIR.is_dir() else []
    for path in agent_files:
        text = path.read_text()
        name = path.stem
        reviewer = is_reviewer(name, roster, text)
        wanted = _stamp_shared(text, _reviewer_variant(name))
        if reviewer:
            try:
                wanted = _stamp_region(wanted, SEVERITY_MARKERS, severity)
                wanted = _stamp_region(wanted, KEYRULE_MARKERS, keyrule)
            except MarkerProblem as exc:
                problems.append(f"{path}: {exc}")
                continue
        if wanted != text:
            drifted.append(_display(path))
            if not check_only:
                path.write_text(wanted)


def _hooks_json() -> tuple[str, list[str]]:
    """The generated ``hooks/hooks.json`` from the rule registry (B4b)."""
    from vsdd.hooks import registry

    return registry.hooks_json(), []


def build(*, check_only: bool = False) -> tuple[list[str], list[str]]:
    """Regenerate everything; return the drifted files and the problems."""
    problems: list[str] = []

    prelude_src = PRELUDE_SRC.read_text()
    wanted_tables = _generate_tables()
    problems.extend(_mapping_problems())
    wanted_schemas = _generate_schemas()
    problems.extend(_hand_literal_problems(PRELUDE_SRC, prelude_src, [TABLE_MARKERS, SCHEMA_MARKERS]))
    try:
        assembled = _replace_between(
            _replace_between(prelude_src, TABLE_MARKERS, wanted_tables),
            SCHEMA_MARKERS,
            wanted_schemas,
        )
    except MarkerProblem as exc:
        problems.append(f"{PRELUDE_SRC.relative_to(NEXT)}: {exc}")
        assembled = prelude_src

    drifted: list[str] = []

    if assembled != prelude_src:
        drifted.append(str(PRELUDE_SRC.relative_to(NEXT)))
        if not check_only:
            PRELUDE_SRC.write_text(assembled)

    for path in _workflow_files():
        text = path.read_text()
        problems.extend(_hand_literal_problems(path, text, [PRELUDE_MARKERS]))
        try:
            wanted = generate_workflow_body(path, assembled)
        except MarkerProblem as exc:
            problems.append(f"{path}: {exc}")
            continue
        # The layout rule judges the post-insert text, so a fresh workflow
        # passes it too (U3-R8).
        problems.extend(meta_ok(wanted, path))
        if wanted != text:
            drifted.append(_display(path))
            if not check_only:
                path.write_text(wanted)

    _stamp_agents(problems, drifted, check_only)

    # Step 5 (B4b): hooks.json, generated from the rule registry.
    try:
        wanted_hooks, hook_problems = _hooks_json()
        problems.extend(hook_problems)
        hooks_path = PLUGIN / "hooks" / "hooks.json"
        if not hooks_path.exists() or hooks_path.read_text() != wanted_hooks:
            drifted.append("plugins/vsdd-factory/hooks/hooks.json")
            if not check_only:
                hooks_path.parent.mkdir(parents=True, exist_ok=True)
                hooks_path.write_text(wanted_hooks)
    except Exception as exc:  # a registry fault names itself at the build
        problems.append(f"hooks/hooks.json: the registry could not be generated: {exc}")

    # Guard: a stamped prelude marker appears only in files the build stamps,
    # scanned over the whole of next/ (excluding virtual environments and
    # caches), looking for both the opening and the closing marker.
    allowed = {p.resolve() for p in _workflow_files()} | {PRELUDE_SRC.resolve()}
    allowed |= {NEXT / "tools" / "build.py", NEXT / "tools" / "check.py"}
    # The harness strips stamped regions by indexOf, and the build's own
    # regression tests quote the marker strings in fixtures.
    allowed |= {
        NEXT / "tests" / "workflows" / "harness.mjs",
        NEXT / "tests" / "workflows" / "universal.test.mjs",
        NEXT / "tests" / "lint" / "test_build.py",
    }
    for path in sorted(NEXT.rglob("*")):
        if not path.is_file() or path.suffix not in (".js", ".mjs", ".md", ".json", ".py", ".toml", ".sh", ".yml", ".yaml"):
            continue
        if any(part in (".venv", "node_modules", "__pycache__", ".pytest_cache", ".ruff_cache", ".git") for part in path.parts):
            continue
        if path.resolve() in allowed:
            continue
        try:
            text = path.read_text()
        except (UnicodeDecodeError, OSError):
            continue
        for marker in PRELUDE_MARKERS:
            if marker in text:
                problems.append(f"{path}: stamped marker outside the stamping scope")

    return sorted(set(drifted)), problems


def main(argv: list[str]) -> int:
    check_only = "--check" in argv
    drifted, problems = build(check_only=check_only)
    for line in problems:
        print(f"ERROR {line}")
    if check_only:
        if problems:
            print(f"build --check: {len(problems)} problem(s), see the ERROR lines above")
            return 1
        if drifted:
            for name in drifted:
                print(f"DRIFT {name}")
            print(f"build --check: {len(drifted)} file(s) drifted; run tools/build.py")
            return 1
        print("build --check: all generated copies are current")
        return 0
    for name in drifted:
        print(f"WROTE {name}")
    print(f"build: {len(drifted)} file(s) regenerated")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
