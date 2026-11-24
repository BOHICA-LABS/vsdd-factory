#!/bin/sh
# Guard: a missing uv must fail loudly. Without this the hook would exit 127,
# which Claude Code treats as non-blocking, and every gate would silently switch off.
# FSPIKE_FAKE_NO_UV=1 simulates a machine without uv.
if [ "${FSPIKE_FAKE_NO_UV:-0}" = "1" ] || ! command -v uv >/dev/null 2>&1; then
  echo "fspike: uv is not installed, so the factory gates cannot run. Install uv and retry." >&2
  exit 2
fi
exec uv run --quiet --script "${CLAUDE_PLUGIN_ROOT}/hooks/runner.py"
