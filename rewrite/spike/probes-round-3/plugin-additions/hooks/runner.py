# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Spike hook runner, round 3: logs what each hook event carries and applies probe rules."""
import json
import os
import sys


def allow_json() -> str:
    return json.dumps(
        {
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "allow",
                "permissionDecisionReason": "fspike auto-allow probe",
            }
        }
    )


def emit_pretool(decision: str, updated_input=None, reason: str = "fspike probe") -> None:
    """Emit a PreToolUse decision, with updatedInput in both the nested and top-level slot
    so the probe does not depend on which spelling this build honours."""
    spec = {
        "hookEventName": "PreToolUse",
        "permissionDecision": decision,
        "permissionDecisionReason": reason,
    }
    if updated_input is not None:
        spec["updatedInput"] = updated_input
    out_obj = {"hookSpecificOutput": spec}
    print(json.dumps(out_obj))


def main() -> int:
    raw = sys.stdin.read()
    try:
        event = json.loads(raw) if raw.strip() else {}
    except json.JSONDecodeError:
        event = {"_unparsed": raw[:200]}

    name = event.get("hook_event_name", "")
    tool = event.get("tool_name", "")
    tool_input = event.get("tool_input") or {}
    agent_type = str(event.get("agent_type", ""))
    agent_id = str(event.get("agent_id", ""))
    session_id = str(event.get("session_id", ""))
    command = str(tool_input.get("command", ""))

    log_path = os.environ.get("FSPIKE_LOG")
    log_dir = os.path.dirname(log_path) if log_path else ""

    record = {
        "event": name,
        "tool": tool,
        "agent_type": agent_type,
        "agent_id": agent_id,
        "session_id": session_id,
        "tool_use_id": str(event.get("tool_use_id", "")),
        "source": event.get("source", ""),
        "trigger": event.get("trigger", ""),
        "command": command[:120],
        "file_path": str(tool_input.get("file_path", ""))[-60:],
        "cwd": str(event.get("cwd", ""))[-70:],
        "subagent_type": str(tool_input.get("subagent_type", "")),
        "keys": sorted(event.keys()),
        "pid": os.getpid(),
    }

    def log() -> None:
        if log_path:
            with open(log_path, "a", encoding="utf-8") as fh:
                fh.write(json.dumps(record) + "\n")

    if name == "SessionStart":
        log()
        source = event.get("source", "unknown")
        briefing = f"FSPIKE-BRIEFING source={source} phase=demo"
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": briefing}}))
        return 0

    # Round-3 rule (item 27): rewrite a Bash command through updatedInput, with allow.
    if name == "PreToolUse" and tool == "Bash" and "PROBE_REWRITE_MARKER" in command:
        record["verdict"] = "rewritten+allow"
        log()
        emit_pretool("allow", {"command": "VSDD_CALLER=ticket-42 " + command}, "fspike rewrite probe")
        return 0

    # Round-3 rule (P3COV-07): rewrite paired with an ask decision.
    if name == "PreToolUse" and tool == "Bash" and "ASKME_REWRITE_MARKER" in command:
        record["verdict"] = "rewritten+ask"
        log()
        emit_pretool("ask", {"command": "VSDD_CALLER=ticket-42 " + command}, "fspike rewrite-ask probe")
        return 0

    # Round-3 rule (item 27 tail): rewrite one field of a StructuredOutput step result.
    if (
        name == "PreToolUse"
        and tool == "StructuredOutput"
        and isinstance(tool_input, dict)
        and tool_input.get("output") == "ORIGVAL"
    ):
        new_input = dict(tool_input)
        new_input["output"] = "ORIGVAL-HOOKREWRITE"
        record["verdict"] = "so-rewritten"
        log()
        emit_pretool("allow", new_input, "fspike structured-output rewrite probe")
        return 0

    # Round-3 rule (item 30): allow the Workflow tool itself.
    if name == "PreToolUse" and tool == "Workflow":
        record["verdict"] = "workflow-allowed"
        log()
        emit_pretool("allow", None, "fspike workflow-permission probe")
        return 0

    if name == "UserPromptExpansion":
        text = raw
        if "BLOCKME" in text:
            record["verdict"] = "blocked"
            log()
            print("fspike: decision D1 recorded by the hook. HOOK-ONLY-TEXT-4471 must not reach the model.", file=sys.stderr)
            return 2
        if "decide" in text:
            record["verdict"] = "context-added"
            log()
            print(
                json.dumps(
                    {
                        "hookSpecificOutput": {
                            "hookEventName": "UserPromptExpansion",
                            "additionalContext": "FSPIKE-DECISION recorded by hook: D1 = yes",
                        }
                    }
                )
            )
            return 0

    # Round-2 rule (item 1): block a bad step result and make the agent return again.
    if name == "PreToolUse" and tool == "StructuredOutput" and "BADQUOTE" in json.dumps(tool_input):
        record["verdict"] = "blocked"
        log()
        print(
            "fspike grounding: the quote is not in the file. Use the exact text GOODQUOTE instead and return your result again.",
            file=sys.stderr,
        )
        return 2

    # Round-2 rule (item 3): refuse a premature stop.
    if agent_type.endswith("closer") and log_dir and agent_id:
        marker = os.path.join(log_dir, f"closed-{agent_id}")
        tries = os.path.join(log_dir, f"stoptries-{agent_id}")
        if name == "PreToolUse" and tool == "Bash" and "fspike-tool close" in command:
            open(marker, "w").close()
        if name == "SubagentStop" and not os.path.exists(marker):
            count = 0
            if os.path.exists(tries):
                count = int(open(tries).read().strip() or "0")
            if count < 3:
                with open(tries, "w") as fh:
                    fh.write(str(count + 1))
                record["verdict"] = f"stop-refused-{count + 1}"
                log()
                print("fspike: you may not stop yet. Run the Bash command `fspike-tool close` first, then stop.", file=sys.stderr)
                return 2

    # Round-2 rule (item 7): hook-approved call not covered by any permission rule.
    if name == "PreToolUse" and tool == "Bash" and "allowme" in command:
        record["verdict"] = "auto-allowed"
        log()
        print(allow_json())
        return 0

    log()

    if name == "PreToolUse" and tool == "Bash" and "SPIKE_BLOCK_ME" in command:
        print("fspike gate: commands containing SPIKE_BLOCK_ME are blocked by the Python runner.", file=sys.stderr)
        return 2

    return 0


if __name__ == "__main__":
    sys.exit(main())
