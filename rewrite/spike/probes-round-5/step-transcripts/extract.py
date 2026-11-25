#!/usr/bin/env python3
"""Keep the tool records of a step transcript.

Usage: extract.py <agent-*.jsonl> <output.jsonl>

Writes one line per record of the transcript that holds a `tool_use` or a `tool_result` entry at any
depth: {"line": <1-based line number in the transcript>, "record": <the record, whole and unchanged>}.
Every other record is left out (the step's prompt, attachments, request shapes and request bodies, which
carry the person's user-level CLAUDE.md). The last line is {"transcript": <file name>, "records": <count of
records in the transcript>, "kept": <count kept>}. Standard library only.
"""
import json
import sys
from pathlib import Path


def holds_tool_entry(node) -> bool:
    if isinstance(node, dict):
        if node.get("type") in ("tool_use", "tool_result"):
            return True
        return any(holds_tool_entry(value) for value in node.values())
    if isinstance(node, list):
        return any(holds_tool_entry(value) for value in node)
    return False


def main() -> int:
    source, target = Path(sys.argv[1]), Path(sys.argv[2])
    total = kept = 0
    with source.open() as lines, target.open("w") as out:
        for number, line in enumerate(lines, 1):
            total += 1
            record = json.loads(line)
            if holds_tool_entry(record):
                kept += 1
                out.write(json.dumps({"line": number, "record": record}, ensure_ascii=False) + "\n")
        out.write(json.dumps({"transcript": source.name, "records": total, "kept": kept}) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
