"""Minimal MCP stdio server for probe round 3: one tool, ping."""
import json
import sys


def send(msg: dict) -> None:
    sys.stdout.write(json.dumps(msg) + "\n")
    sys.stdout.flush()


def main() -> None:
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        req = json.loads(line)
        method = req.get("method", "")
        rid = req.get("id")
        if method == "initialize":
            send(
                {
                    "jsonrpc": "2.0",
                    "id": rid,
                    "result": {
                        "protocolVersion": req.get("params", {}).get("protocolVersion", "2024-11-05"),
                        "capabilities": {"tools": {}},
                        "serverInfo": {"name": "tiny", "version": "0.0.1"},
                    },
                }
            )
        elif method.startswith("notifications/"):
            pass
        elif method == "tools/list":
            send(
                {
                    "jsonrpc": "2.0",
                    "id": rid,
                    "result": {
                        "tools": [
                            {
                                "name": "ping",
                                "description": "Answers PONG with the echo.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {"echo": {"type": "string"}},
                                },
                            }
                        ]
                    },
                }
            )
        elif method == "tools/call":
            args = req.get("params", {}).get("args", {}) or {}
            send(
                {
                    "jsonrpc": "2.0",
                    "id": rid,
                    "result": {"content": [{"type": "text", "text": f"TINY-PONG {json.dumps(args, sort_keys=True)}"}]},
                }
            )
        elif rid is not None:
            send({"jsonrpc": "2.0", "id": rid, "error": {"code": -32601, "message": "no such method"}})


if __name__ == "__main__":
    main()
