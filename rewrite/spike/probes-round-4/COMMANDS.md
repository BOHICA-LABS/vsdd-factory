# Platform probes, round 4 — method and commands

2026-11-24, Claude Code 2.1.331, macOS (Darwin 25.5.0). Scratch plugin and project under the session
scratchpad (`pr4/`), probe projects under the Claude Code temp tree, headless `claude -p` throughout. All
sessions ran with the throwaway plugin `fs4` (a renamed copy of `rewrite/spike/plugin`, stripped to agents,
hooks and probe workflows) — `plugin/` here is the exact copy that ran. Each session wrote its hook log
(`FSPIKE_LOG`), its `--output-format json` result and stderr into the folder named for the question. The hook
runner logs every `PreToolUse` and `PostToolUse` event it sees (tool, `agent_type`, `agent_id`,
`permission_mode`, input, response, verdict) and applies the probe rules of Q2 to Q5 and Q11a.

Driver: every probe session ran `/fs4:probecall <workflow>`, a typed skill whose expansion tells the main
session to start the saved workflow with the Workflow tool, passing only the name. Two probe facts on the way
(recorded under q1-q5-followup as s1 and q7-long as s7): a main session that calls the Workflow tool unguided
may pass its own inline `script` instead of the named workflow (the tool accepts it and the named file never
runs), and in the default permission mode a compound command needs every part allowed, else the part outside
the allowlist draws "This command requires approval".

Sessions (all `--model sonnet --effort low --max-budget-usd 2`):

```
cd <pr4>/project && FSPIKE_LOG=<pr4>/logs/s<N>.jsonl claude -p "/fs4:probecall fs4:<wf>" \
  --plugin-dir <pr4>/plugin --permission-mode <MODE> \
  --allowedTools <list> --output-format json </dev/null > <pr4>/logs/<s>-result.json 2> <pr4>/logs/<s>-stderr.txt
```

| Folder | Session | Mode | Workflow | allowedTools beyond Workflow/Agent | Question |
|---|---|---|---|---|---|
| q1-q5-followup | s1, s1c, smoke | default | main, followup | `Bash(echo *)`, `Bash(echo:*)`, s1 also `Bash(sh *)`, `Bash(sh:*)` | Q1 Q2 Q3 Q5 |
| q4-decisionless | s2b | default | q4 | `Bash(sh *)`, `Bash(sh:*)` | Q4 default |
| q4-auto | s3b | auto | q4 | `Bash(sh *)`, `Bash(sh:*)` | Q4 auto |
| q7-long | s7 (+ first attempt, superseded) | default | q7 | `Bash(echo *)`, `Bash(echo:*)`, then `Bash(python3 *)` | Q7 |
| q8-sandbox | s4b | default, `--settings '{"sandbox":{"enabled":true}}'` | sandbox | `Bash(sh *)`, `Bash(python3 *)` | Q6, Q8 sandbox-on |
| q8-detach | s5b | default (no sandbox) | detach-off | `Bash(python3 *)` | Q8 sandbox-off |
| q9-glob | s6b | default | q9 | none | Q9 |
| q11-expansion-block | s11a, s11b | default | (typed command, q11) | `Bash(python3 *)` | Q11a, Q11b |
| q11a-control | s11ac | default | q9 | none | Q11a control |
| q10-git-hooks | (no claude) | — | — | — | Q10, plain git |

Probe scripts the steps ran (in the scratch project): `probe.sh` (prints `CALLER=${VSDD_CALLER:-unset}`);
`spawn.py` (double fork, setsid; grandchild sleeps then writes its output file); `spawncall.py` (spawn then
block past the tool timeout); `wait590.py` (prints P7START, sleeps 590 s, prints P7LONGDONE); `sbx.sh` (home
write canary, `git worktree add .worktrees/x -b story/x`, commit there, `git update-ref`, worktree list);
`waitslow.py` (sleeps 120 s). `.factory` in the project is a worktree of an orphan branch, listed in
`.gitignore`, holding `specs/contract.md` with the marker `CONTRACT-MARKER-9`.

Q10 ran plain git only: two wirings (`.git/hooks/pre-commit`; `core.hooksPath .githooks`), each repo with a
divergent `feature` branch and a second worktree `.worktrees/_integration`; a hook logging `pwd`, `GIT_DIR`,
`GIT_WORK_TREE`, `git rev-parse --show-toplevel`, a tracked file and an untracked file that exists only in the
main checkout, then failing when it is missing. Auto `git merge --no-ff` compared with `git merge --no-commit`
followed by an explicit `git commit`. Repos kept under `pr4-q10.YhrECl/` in the scratchpad (temporary).

Kill test (q11-expansion-block, s11b): a session running `fs4:q11` (a step whose Bash call sleeps 120 s) got
`kill -TERM` to the session process 40 s in; process table sampled before and 4 s after.
