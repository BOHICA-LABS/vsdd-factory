#!/bin/sh
# Guard: a missing uv must fail loudly. Without this the hook would exit 127,
# which Claude Code treats as non-blocking, and every gate would silently switch off.
if ! command -v uv >/dev/null 2>&1; then
  echo "p5: uv is not installed, so the probe hook cannot run. Install uv and retry." >&2
  exit 2
fi
exec uv run --quiet --script "${CLAUDE_PLUGIN_ROOT}/hooks/runner.py"
