#!/bin/sh
# usage: run.sh <name> <workflow> <mode> <settings-json-or-empty> <allowedTools...>
C=/private/tmp/claude-501/-Users-jmagady-dev2-vsdd-factory/f468fc16-2190-4e04-8bbb-50654bb31906/scratchpad/chk4
n=$1; wf=$2; mode=$3; st=$4; shift 4
cd $C/project || exit 1
date +%s > $C/logs/$n.start
if [ -n "$st" ]; then set -- --settings "$st" --allowedTools "$@"; else set -- --allowedTools "$@"; fi
CHK_LOG=$C/logs/$n.jsonl claude -p "/chk4:probecall chk4:$wf" --plugin-dir $C/plugin --permission-mode $mode \
  --model sonnet --effort low --max-budget-usd 2 --output-format json "$@" </dev/null > $C/logs/$n-result.json 2> $C/logs/$n-stderr.txt
echo "rc=$?" >> $C/logs/$n-stderr.txt
date +%s > $C/logs/$n.end
