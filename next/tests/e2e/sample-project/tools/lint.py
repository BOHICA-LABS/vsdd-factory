#!/usr/bin/env python3
"""Static checks for the tally sample.

Default mode is lint: every module must parse, and module-level imports
must be used. `--format` switches to the format check: space indentation,
no trailing whitespace, at most 100 columns, final newline.

Both modes print `path:line: message` for each finding and exit 1 when
there is at least one, 0 otherwise. Standard library only.
"""

import argparse
import ast
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent
CHECKED_DIRS = ("tally", "tests", "tools")
MAX_LINE_LENGTH = 100


def iter_python_files():
    for name in CHECKED_DIRS:
        directory = PROJECT_ROOT / name
        if directory.is_dir():
            yield from sorted(directory.rglob("*.py"))


def dunder_all_names(tree):
    for node in tree.body:
        if isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name) and target.id == "__all__":
                    return [elt.value for elt in node.value.elts]
    return []


def lint_file(path, text):
    findings = []
    try:
        tree = ast.parse(text, filename=str(path))
    except SyntaxError as exc:
        return [(path, exc.lineno or 1, f"syntax error: {exc.msg}")]

    imported = {}
    for node in tree.body:
        if isinstance(node, ast.Import):
            for alias in node.names:
                imported[alias.asname or alias.name.split(".")[0]] = node.lineno
        elif isinstance(node, ast.ImportFrom):
            for alias in node.names:
                if alias.name != "*":
                    imported[alias.asname or alias.name] = node.lineno

    used = {node.id for node in ast.walk(tree) if isinstance(node, ast.Name)}
    used.update(dunder_all_names(tree))
    if path.name != "__init__.py":  # an __init__ re-exports its imports
        for name, lineno in imported.items():
            if name not in used:
                findings.append((path, lineno, f"unused import: {name}"))
    return findings


def format_file(path, text):
    findings = []
    for lineno, line in enumerate(text.splitlines(True), 1):
        body = line.rstrip("\n").rstrip("\r")
        indent = body[: len(body) - len(body.lstrip())]
        if "\t" in indent:
            findings.append((path, lineno, "tab used for indentation"))
        if body != body.rstrip():
            findings.append((path, lineno, "trailing whitespace"))
        if len(body) > MAX_LINE_LENGTH:
            findings.append((path, lineno, f"line longer than {MAX_LINE_LENGTH} columns"))
    if text and not text.endswith("\n"):
        findings.append((path, len(text.splitlines()), "missing final newline"))
    return findings


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--format", action="store_true", help="run the format check only")
    args = parser.parse_args()

    checker = format_file if args.format else lint_file
    label = "format" if args.format else "lint"
    checked = 0
    all_findings = []
    for path in iter_python_files():
        text = path.read_text(encoding="utf-8")
        all_findings.extend(checker(path, text))
        checked += 1

    for path, lineno, message in all_findings:
        print(f"{path.relative_to(PROJECT_ROOT)}:{lineno}: {message}")
    if all_findings:
        print(f"{label}: {len(all_findings)} finding(s) in {checked} files")
        return 1
    print(f"{label} ok: {checked} files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
