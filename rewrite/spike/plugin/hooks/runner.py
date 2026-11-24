# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Spike hook runner: logs what each hook event carries and applies a few probe rules."""
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


def main() -> int:
    raw = sys.stdin.read()
    try:
        event = json.loads(raw) if raw.strip() else {}
    except json.JSONDecodeError:
        event = {"_unparsed": raw[:200]}

    name = event.get("hook_event_name", "")
    tool = event.get("tool_name", "")
    tool_input = event.get("tool_input") or {}
    agent_type = event.get("agent_type", "")
    agent_id = event.get("agent_id", "")
    command = str(tool_input.get("command", ""))

    log_path = os.environ.get("FSPIKE_LOG")
    log_dir = os.path.dirname(log_path) if log_path else ""

    record = {
        "event": name,
        "tool": tool,
        "agent_type": agent_type,
        "agent_id": agent_id,
        "source": event.get("source", ""),
        "trigger": event.get("trigger", ""),
        "command": command[:80],
        "file_path": str(tool_input.get("file_path", ""))[-60:],
        "cwd": str(event.get("cwd", ""))[-70:],
        "subagent_type": str(tool_input.get("subagent_type", "")),
        "skill": str(tool_input.get("skill", "")),
        "keys": sorted(event.keys()),
        "env_plugin_data": bool(os.environ.get("CLAUDE_PLUGIN_DATA")),
        "env_project_dir": bool(os.environ.get("CLAUDE_PROJECT_DIR")),
    }
    # Payload shapes worth seeing in full (truncated).
    if name in ("UserPromptExpansion", "UserPromptSubmit") or tool == "StructuredOutput":
        record["raw"] = raw[:900]

    def log() -> None:
        if log_path:
            with open(log_path, "a", encoding="utf-8") as fh:
                fh.write(json.dumps(record) + "\n")

    if name == "SessionStart":
        log()
        source = event.get("source", "unknown")
        briefing = f"FSPIKE-BRIEFING source={source} phase=demo next=/fspike:wave token=TOKEN-{source.upper()}-7Q4"
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": briefing}}))
        return 0

    # Probe 2: the decision command a person types.
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

    # Probe 1: the step result as the hook sees it, and whether a block makes the agent return again.
    if name == "PreToolUse" and tool == "StructuredOutput" and "BADQUOTE" in json.dumps(tool_input):
        record["verdict"] = "blocked"
        log()
        print(
            "fspike grounding: the quote is not in the file. Use the exact text GOODQUOTE instead and return your result again.",
            file=sys.stderr,
        )
        return 2

    # Probe 3: an agent may not stop before it has run its closing command.
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
                print(
                    "fspike: you may not stop yet. Run the Bash command `fspike-tool close` first, then stop.",
                    file=sys.stderr,
                )
                return 2

    # Probe 7: the hook approves a call that no permission rule allows.
    if name == "PreToolUse" and tool == "Bash" and "allowme" in command:
        record["verdict"] = "auto-allowed"
        log()
        print(allow_json())
        return 0

    log()

    if name == "PreToolUse" and tool == "Bash" and "SPIKE_BLOCK_ME" in command:
        print("fspike gate: commands containing SPIKE_BLOCK_ME are blocked by the Python runner.", file=sys.stderr)
        return 2

    # Information wall keyed on the calling agent: builders may not read the holdout file.
    if (
        name == "PreToolUse"
        and tool == "Read"
        and "holdout-secret" in str(tool_input.get("file_path", ""))
        and agent_type.endswith("builder")
    ):
        print(f"fspike wall: agent '{agent_type}' may not read holdout material.", file=sys.stderr)
        return 2

    return 0


if __name__ == "__main__":
    sys.exit(main())
