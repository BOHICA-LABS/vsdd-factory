#!/bin/sh
# Leg D: rerun A of the round-4 check again (items 36 to 38 and 40), Claude Code 2.1.332.
# Conditions as in check-reruns: default mode, allowedTools Workflow and Bash(echo *) only.
S=/Users/jmagady/.vsdd-probes/round-5.QkGwwK
cd "$S/projd" || exit 1
CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 P5_LOG="$S/logs/D.jsonl" CHK_RULES=1 \
claude -p "/chk5:r123 $S/chk5plugin" \
  --plugin-dir "$S/chk5plugin" \
  --permission-mode default \
  --allowedTools "Workflow Bash(echo *)" \
  --settings '{"sandbox":{"enabled":false}}' \
  --setting-sources project,local --strict-mcp-config \
  --model sonnet --effort low --max-budget-usd 2 \
  --output-format json < /dev/null > "$S/logs/D-result.json" 2> "$S/logs/D-stderr.txt"
rc=$?
echo "rc=$rc" > "$S/logs/D-rc.txt"
echo "D done rc=$rc"
