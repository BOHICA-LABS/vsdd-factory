#!/bin/sh
# Leg A: one headless session started by the typed workflow command itself.
S=/Users/jmagady/.vsdd-probes/round-5.QkGwwK
cd "$S/project" || exit 1
CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 P5_LOG="$S/logs/A.jsonl" \
claude -p "/p5:one $S/plugin" \
  --plugin-dir "$S/plugin" \
  --permission-mode default \
  --allowedTools "Workflow Agent Bash Read Write Edit Glob Grep" \
  --settings '{"sandbox":{"enabled":false}}' \
  --setting-sources project,local --strict-mcp-config \
  --model sonnet --effort medium --max-budget-usd 1 \
  --output-format json < /dev/null > "$S/logs/A-result.json" 2> "$S/logs/A-stderr.txt"
rc=$?
echo "rc=$rc" > "$S/logs/A-rc.txt"
echo "A done rc=$rc"
