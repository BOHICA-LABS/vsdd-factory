#!/usr/bin/env python3
"""Check every line that items 52 and 55 of topics/platform-proven-3.md cite against the kept tool records.

Usage: verify.py <the step-transcripts folder>. Prints one line per check; exit 1 if any fails.
"""
import json
import os
import sys

folder = sys.argv[1]
failed = []


def load(name):
    records = {}
    for line in open(os.path.join(folder, name)):
        entry = json.loads(line)
        if "line" in entry:
            records[entry["line"]] = entry["record"]
    return records


def entries(record, kind):
    found = []

    def walk(node):
        if isinstance(node, dict):
            if node.get("type") == kind:
                found.append(node)
            for value in node.values():
                walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)

    walk(record)
    return found


def check(label, ok):
    print(("ok   " if ok else "FAIL ") + label)
    if not ok:
        failed.append(label)


def row(name, line):
    record = load(name)[line]
    result = entries(record, "tool_result")[0]
    decision = record.get("permissionDecision") or {}
    return (record["type"], result.get("is_error"), decision.get("decision"), decision.get("source"),
            decision.get("reasonCode"), record.get("toolDenialKind"), str(result.get("content")))


A = "leg-A-agent-a3338c7a916b05723.jsonl"
RA = "rerun-A-agent-afbe9cef34f0bc21c.jsonl"
CTRL = "leg-D-R3CTRL-agent-a5caa4d535b10df94.jsonl"
R5B = "leg-D-R5B-agent-a4bd7d5edd93227d3.jsonl"
R2C = "leg-D-R2C-agent-ac674367fa72bb811.jsonl"
R3A = "leg-D-R3A-agent-a48bb837317fa3653.jsonl"
T1 = "rerun-B-2c-agent-aeb3de9ef0ed5cf90.jsonl"

r = row(CTRL, 15)
check("item 52 row 1: R3CTRL line 15 is reject, config, bashMiss:no-rule-match, user-rejected, 'requires approval'",
      r[:6] == ("user", True, "reject", "config", "bashMiss:no-rule-match", "user-rejected") and "requires approval" in r[6])
r = row(R5B, 15)
check("item 52 row 2: R5B line 15 is reject, hook, hook:PreToolUse, permission-rule",
      r[:6] == ("user", True, "reject", "hook", "hook:PreToolUse", "permission-rule"))
for name in (A, RA):
    users = [rec for rec in load(name).values() if rec["type"] == "user"]
    check(f"item 52 row 3: every result of {name} is accept from config, is_error false or absent, no toolDenialKind",
          bool(users) and all(rec["permissionDecision"]["decision"] == "accept" and rec["permissionDecision"]["source"] == "config"
                              and "toolDenialKind" not in rec and not entries(rec, "tool_result")[0].get("is_error") for rec in users))
r = row(R3A, 16)
check("item 52 row 3: R3A line 16 is accept from a hook, is_error false", r[:6] == ("user", False, "accept", "hook", None, None))
for line in (16, 23, 30, 37, 44):
    r = row(R2C, line)
    check(f"item 52 row 4: R2C line {line} is accept from a hook with is_error true",
          r[:6] == ("user", True, "accept", "hook", None, None) and "does not match required schema" in r[6])
r = row(T1, 15)
check("item 52 row 5: the 2-cent rerun's line 15 has no permissionDecision, user-rejected, is_error true",
      r[:6] == ("user", True, None, None, None, "user-rejected") and "doesn't want to proceed" in r[6])


def copies(name, kind):
    seen = {}
    for line, record in sorted(load(name).items()):
        for entry in entries(record, kind):
            seen.setdefault(entry.get("id") or entry.get("tool_use_id"), []).append((line, record["type"]))
    return seen


for kind in ("tool_use", "tool_result"):
    seen = copies(A, kind)
    check(f"item 55: leg A has 8 {kind} ids in 10 entries", len(seen) == 8 and sum(len(v) for v in seen.values()) == 10)
seen = copies(A, "tool_result")
check("item 55: leg A line 30 (api-request-blob) repeats the results of lines 25 and 27",
      sorted(v for v in seen.values() if len(v) > 1) == [[(25, "user"), (30, "api-request-blob")], [(27, "user"), (30, "api-request-blob")]])
seen = copies(RA, "tool_result")
check("item 55: rerun A line 28 (api-request-blob) repeats the results of lines 23 and 25",
      sorted(v for v in seen.values() if len(v) > 1) == [[(23, "user"), (28, "api-request-blob")], [(25, "user"), (28, "api-request-blob")]])
for name, line in ((A, 30), (RA, 28)):
    record = load(name)[line]
    check(f"item 55: the copy at {name} line {line} has message.role user and none of item 52's keys",
          record["message"]["role"] == "user" and "permissionDecision" not in record and "toolDenialKind" not in record)
for name in (CTRL, R5B, R2C, T1):
    check(f"item 55: {name} holds no copy (assistant and user records only)",
          {rec["type"] for rec in load(name).values()} == {"assistant", "user"})

sys.exit(1 if failed else 0)
