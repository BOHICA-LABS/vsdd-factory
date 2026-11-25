# Warm-up prompt

Paste the block below after a compaction, or as the first message of a new session, to reload what is
needed to continue the 1.0 rewrite.

```
We are continuing the vsdd-factory 1.0 rewrite on branch rewrite/v1. Warm up before doing anything else.

1. Read rewrite/STATE.md in full. It is the resume record and it wins over anything you remember.
2. Read rewrite/spec/README.md, the section "The thin slice" in rewrite/spec/packaging/build-plan.md, and
   the ORDER.md of the step STATE.md says is being built or is next (rewrite/plans/step-3/ORDER.md and
   rewrite/plans/step-4/ORDER.md at the time this was written).
3. Run these and read what they print:
   git status --short | awk '{print $1}' | sort | uniq -c ; git log --oneline -3
   python3 rewrite/tools/findings.py build | head -3
   python3 rewrite/tools/spec_lint.py | tail -1
   (cd next && uv run --no-sync python tools/check.py | tail -1 && uv run --no-sync python -m pytest -q | tail -1)
   ls rewrite/findings/outcomes | tail -30 ; ls rewrite/findings/results ; ls rewrite/workflows
4. For each workflow that STATE.md lists as in flight, work out its state from the "Done when" column:
   still running in this session, finished (its outputs are on disk), or lost with an earlier session and
   needing a relaunch from the script in rewrite/workflows/ with the same args. Do not read a workflow's raw
   output file; read the files it wrote.
5. Tell me in a few lines where we are, what is running, what you will do next, and what you need from me.
   Then carry on with the next action in STATE.md without waiting, unless something there is waiting on me.

Rules that always apply: Fable at extra-high effort plans, Sonnet at extra-high effort writes code, Opus at
extra-high effort reviews; use a workflow whenever one applies; commit or push only when I ask; no
attribution lines of any kind in commit messages; mirror a new capability back to me before designing it;
keep rewrite/STATE.md current at every step boundary.
```
