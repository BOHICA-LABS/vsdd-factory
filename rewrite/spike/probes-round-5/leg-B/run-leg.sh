#!/bin/sh
# Leg B: a three-step workflow under a 0.05 dollar cap, same flags otherwise.
S=/Users/jmagady/.vsdd-probes/round-5.QkGwwK
cd "$S/project" || exit 1
CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 P5_LOG="$S/logs/B.jsonl" \
claude -p "/p5:three $S/plugin" \
  --plugin-dir "$S/plugin" \
  --permission-mode default \
  --allowedTools "Workflow Agent Bash Read Write Edit Glob Grep" \
  --settings '{"sandbox":{"enabled":false}}' \
  --setting-sources project,local --strict-mcp-config \
  --model sonnet --effort medium --max-budget-usd 0.05 \
  --output-format json < /dev/null > "$S/logs/B-result.json" 2> "$S/logs/B-stderr.txt"
rc=$?
echo "rc=$rc" > "$S/logs/B-rc.txt"
echo "B done rc=$rc"
