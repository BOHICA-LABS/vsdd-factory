# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Round-4 probe hook runner: logs every event; applies the Q2-Q5 rewrite/refuse rules."""
import json
import os
import sys


def out(obj: dict) -> None:
    print(json.dumps(obj))


def log(record: dict, path_env: str = "FSPIKE_LOG") -> None:
    path = os.environ.get(path_env)
    if path:
        with open(path, "a", encoding="utf-8") as fh:
            fh.write(json.dumps(record) + "\n")


def main() -> int:
    raw = sys.stdin.read()
    try:
        event = json.loads(raw) if raw.strip() else {}
    except json.JSONDecodeError:
        event = {"_unparsed": raw[:200]}

    name = event.get("hook_event_name", "")
    tool = event.get("tool_name", "")
    tool_input = event.get("tool_input") or {}
    inp = dict(tool_input) if isinstance(tool_input, dict) else {}

    record = {
        "event": name,
        "tool": tool,
        "agent_type": event.get("agent_type", ""),
        "agent_id": event.get("agent_id", ""),
        "session_id": event.get("session_id", ""),
        "permission_mode": event.get("permission_mode", ""),
        "command": str(inp.get("command", ""))[:100],
        "input_keys": sorted(inp.keys()),
        "input": json.dumps(inp)[:500],
        "response": json.dumps(event.get("tool_response"))[:500] if name == "PostToolUse" else "",
    }

    def emit() -> None:
        log(record)

    # ---- PreToolUse rules -------------------------------------------------
    if name == "UserPromptExpansion":
        if "BLOCKME11" in raw:
            record["verdict"] = "q11a-block-expansion"
            emit()
            print("Q11A-BLOCK: this typed command is refused by the probe hook.", file=sys.stderr)
            return 2
        record["verdict"] = "q11a-expansion-passed"
        emit()
        return 0
    if name == "PreToolUse" and tool == "StructuredOutput":
        blob = json.dumps(inp)
        if "Q2A" in blob:  # add a key the schema does not declare
            if isinstance(inp.get("findings"), list) and inp["findings"]:
                fi = dict(inp["findings"][0])
                fi["grounded"] = False
                inp["findings"] = [fi, *inp["findings"][1:]]
                record["verdict"] = "q2a-added-grounded-false"
                emit()
                out({"hookSpecificOutput": {"hookEventName": "PreToolUse",
                                            "permissionDecision": "allow",
                                            "permissionDecisionReason": "fs4 q2a",
                                            "updatedInput": inp}})
                return 0
        if "Q2B" in blob:  # key declared optional in the schema
            if isinstance(inp.get("findings"), list) and inp["findings"]:
                fi = dict(inp["findings"][0])
                fi["grounded"] = True
                inp["findings"] = [fi, *inp["findings"][1:]]
                record["verdict"] = "q2b-added-grounded-true"
                emit()
                out({"hookSpecificOutput": {"hookEventName": "PreToolUse",
                                            "permissionDecision": "allow",
                                            "permissionDecisionReason": "fs4 q2b",
                                            "updatedInput": inp}})
                return 0
        if "Q3B" in blob:  # rewrite existing field, allow (default mode)
            if isinstance(inp.get("output"), str):
                inp["output"] = inp["output"].replace("Q3BMARKER", "Q3BMARKER-HOOKREWRITE")
                record["verdict"] = "q3b-rewrote-output-allow"
                emit()
                out({"hookSpecificOutput": {"hookEventName": "PreToolUse",
                                            "permissionDecision": "allow",
                                            "permissionDecisionReason": "fs4 q3b",
                                            "updatedInput": inp}})
                return 0
        if inp.get("label") == "Q5B":
            if not inp.get("note"):
                record["verdict"] = "q5b-refuse-empty-note"
                emit()
                print("FS4-HOOK-GATE-9: your result is missing note. Add note = the first 40 characters of this message and return your result again.", file=sys.stderr)
                return 2
            record["verdict"] = "q5b-passed"
            emit()
            return 0
        if inp.get("label") == "Q5C":
            if "note" not in inp:
                record["verdict"] = "q5c-hook-saw-missing-required-key"
                emit()
                return 0
            record["verdict"] = "q5c-input-has-note"
            emit()
            return 0
        if inp.get("label") == "Q5" or inp.get("label") == "Q5R":
            attempts = inp.get("attempts")
            if isinstance(attempts, str):
                record["verdict"] = "q5-refuse-string-attempts"
                emit()
                print("fs4: your result does not match its schema. Return attempts as a NUMBER (e.g. 3), then return your result again.",
                      file=sys.stderr)
                return 2
            record["verdict"] = "q5-passed"
            emit()
            return 0

    if name == "PreToolUse" and tool == "Bash":
        cmd = str(inp.get("command", ""))
        if "RUNME3A" in cmd and "RUNME3ACTRL" not in cmd:
            inp["command"] = "VSDD_CALLER=ticket-42 " + cmd
            record["verdict"] = "q3a-allow-rewrite"
            emit()
            out({"hookSpecificOutput": {"hookEventName": "PreToolUse",
                                        "permissionDecision": "allow",
                                        "permissionDecisionReason": "fs4 q3a",
                                        "updatedInput": inp}})
            return 0
        if "RUNME3ACTRL" in cmd:
            record["verdict"] = "q3a-control-no-decision"
            emit()
            return 0
        if "RUNME4" in cmd:  # updatedInput with NO permissionDecision
            inp["command"] = "VSDD_CALLER=ticket-42 " + cmd
            record["verdict"] = "q4-rewrite-no-decision"
            emit()
            out({"hookSpecificOutput": {"hookEventName": "PreToolUse",
                                        "updatedInput": inp}})
            return 0

    emit()
    return 0


if __name__ == "__main__":
    sys.exit(main())
