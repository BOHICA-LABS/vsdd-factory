# Platform probes, round 5 — method and commands

2026-11-25, Claude Code 2.1.332 (`~/.local/bin/claude`), macOS 26.5.2 (Darwin 25.5.0, arm64), git 2.50.1.
Scratch directory `~/.vsdd-probes/round-5.QkGwwK/` (under the home directory, as ORDER-6 8.2 directs), with
`plugin/` (throwaway plugin `p5`), `project/` (the leg-A/B project), `chk5plugin/` and `projd/` (leg D),
and `logs/`. Each folder here (`leg-A`, `leg-B`, `leg-C`, `leg-D`) holds that leg's commands, result files
and logs. Every session ran from the scratch project's directory, headless, standard input `/dev/null`.

The plugin `p5` (in `leg-A/plugin/`, the exact copy that ran): the round-4 logging hook (runner logs the
WHOLE payload of every event, timestamped, to `P5_LOG`; no permission decisions), agents/worker.md (tools
`Bash, Read, Write, Edit, Glob, Grep`, model sonnet, effort low), `bin/probe-marker` (prints
`BIN-MARKER-R5`), `methods/probe.md` (`METHOD-MARKER-R5`), workflows `one.js` and `three.js`.

The probe project: `git init -b main`, one commit, local identity, signing off; then
`git worktree add .worktrees/x -b story/x`, and in the worktree `notes.md` (holding `TODO-5`) committed on
`story/x`. No `.claude/` directory, no settings file anywhere in the project.

## Leg A (and B, C) — one headless session, the gate

`leg-A/run-leg.sh`; leg B is the same with workflow `three`, cap `0.05`:

```
cd ~/.vsdd-probes/round-5.QkGwwK/project
CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 P5_LOG=<S>/logs/A.jsonl \
claude -p "/p5:one <S>/plugin" \
  --plugin-dir <S>/plugin \
  --permission-mode default \
  --allowedTools "Workflow Agent Bash Read Write Edit Glob Grep" \
  --settings '{"sandbox":{"enabled":false}}' \
  --setting-sources project,local --strict-mcp-config \
  --model sonnet --effort medium --max-budget-usd 1 \
  --output-format json < /dev/null > <S>/logs/A-result.json 2> <S>/logs/A-stderr.txt
```

`<S>` is `/Users/jmagady/.vsdd-probes/round-5.QkGwwK`. Started by the typed workflow command itself, no
skill anywhere in the plugin. Session id (A): `4265e389-8597-4ead-b675-cfdb38acdbfa`; (B):
`654e64c3-4863-4d4f-af29-556e11e815f9`, run from the same project. The step's one prompt is in
`leg-A/platform/one-wf_bb1971a5-1cd.js` (the script field of the run record): run the plugin's bin tool by
bare name and by absolute path, Read `<plugin>/methods/probe.md`, Write `leg5-write.md` and Edit `notes.md`
in `.worktrees/x`, commit there with plain git, return the markers.

Leg C ran no session: `leg-C/LEG-C.md` compares leg A's folder with the kept round-4 folder named in
ORDER-6 8.2.

## Leg D — rerun A of the round-4 check, on 2.1.332

`chk5plugin/` is a reconstruction of the check's `chk4` plugin (round-4 runner rules rebuilt from CHECK.md;
agents `clerk` and `returner`; `workflows/r123.js` recovered byte for byte from the check run record's
`script` field, `agentType` renamed to `chk5:*`, `PROJ` pointed at `projd/`). `projd/` holds `probe.sh`
(`echo "CALLER=${VSDD_CALLER:-unset} ARGS=$1"`). Entered by the typed command, as leg A; the check's own
rerun went through the typed skill `probecall`, so the entry route differs by that much:

```
cd ~/.vsdd-probes/round-5.QkGwwK/projd
CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=0 P5_LOG=<S>/logs/D.jsonl CHK_RULES=1 \
claude -p "/chk5:r123 <S>/chk5plugin" \
  --plugin-dir <S>/chk5plugin \
  --permission-mode default \
  --allowedTools "Workflow Bash(echo *)" \
  --settings '{"sandbox":{"enabled":false}}' \
  --setting-sources project,local --strict-mcp-config \
  --model sonnet --effort low --max-budget-usd 2 \
  --output-format json < /dev/null > <S>/logs/D-result.json 2> <S>/logs/D-stderr.txt
```

Session id: `9e2b33bb-77e5-492a-a52b-def85effd91b`. R3B ran without its round-4 output rewrite (omitted
from the reconstructed runner); the StructuredOutput half of item 38 is carried by R2A/R2B, whose
allow-plus-rewrite results reached the script.

## Files

Per leg: `<leg>.jsonl` (hook log; split `-part1`/`-part2` at an event boundary where over 16 KiB, whole
records kept), `<leg>-result.json` (`--output-format json`), `<leg>-stderr.txt`, `<leg>-rc.txt` (exit
code), `run-leg.sh`, and `platform/` with the session's `workflows/wf_*.json`, its saved
`workflows/scripts/*.js`, the step journal, and the agent meta files. Hook-log lines are single JSON
records and can exceed the 400-character rule; nothing else does. No transcript of any but probe text is
copied here.

Cost observed: A $0.120, B $0.058 (capped), D $0.172; about $0.35 in all.
