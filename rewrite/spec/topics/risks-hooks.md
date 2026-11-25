---
kind: topic
id: risks-hooks
sources: [hooks.md#7]
---

# Hook risks

The risks the hook design names and what mitigates each, taken with the other rules from the hook
design (the sources line above).

1. **Unverified platform behaviour.** The three facts the first spike did not cover were probed on
   2026-11-23. `step-results` and `decisions` are proven as designed. `spike-bounds` is proven in part:
   a refused `SubagentStop` does not hold a workflow step that has already returned its result, so the
   rule holds a step at its `StructuredOutput` call instead. What is still unverified is listed in
   [[topic:hook-platform-facts]]: the command rewrite that carries the caller marker's key, the
   session id inside workflow steps, and the permit for the `Workflow` tool. Each has a named
   fallback, and in every case the tool's own checks at the point of no return still hold.
2. **The shell is not sandboxed.** Walls and write scopes over `Bash` are heuristics. A command such
   as `python -c "open(...)"` passes them, and so could a write to the evidence or decision store.
   Mitigation: walled agents get the narrowest tool set; the tool re-measures at delivery instead of
   trusting a stored record alone ([[topic:risks-pipeline]]); `vsdd validate` re-checks artifacts at
   gates. A real sandbox is out of scope for 1.0.
3. **Fail-closed lockout.** A runner bug, a broken uv, or a missing interpreter blocks every guarded
   tool in every factory project. Mitigation: `vsdd hooks selftest` in CI and in `vsdd doctor`,
   messages that carry the fix, and the emergency switch.
4. **False positives cause workarounds.** A guard that blocks legitimate commands teaches agents to
   route around it. Mitigation: parsed commands, the harvested old cases, `advise` strength for
   trials, and `rule.would_block` events to measure a rule before enforcing it.
5. **Latency and load.** About 55 to 70 ms is added to every guarded call, including every `Read`, and
   up to 16 agents run at once. Mitigation: the inert path costs 8 ms, imports are per event, bytecode
   is cached, and the performance test guards the budget. If `Read` volume proves costly, the `if`
   field can narrow that matcher at the price of weaker walls.
6. **Hooks disabled outside the runner's sight.** `disableAllHooks`, a disabled plugin or a bare
   headless run switches every gate off without any code of ours running. Mitigation: `gate-guard`
   blocks tool edits that would do it, and the survey of every workflow detects it, because its call
   has no caller marker.
7. **Policy tables have three owners.** A row that names no agent file, or a class glob that does not
   match the state layout, silently protects nothing. Mitigation: the policy tests in
   [[topic:rule-testing]].
8. **The main session and ancestor searches.** Until one of the three options in the read-walls rule
   is confirmed ([[rule:read-walls]]), project-wide searches by the main session are blocked in any
   project that has holdout scenarios, which will be felt at once. This needs an answer from the
   state design before the build.
9. **uv changes.** The launch relies on four uv flags and on uv's exit codes. Mitigation: `vsdd
   doctor` runs the exact launch command instead of comparing version numbers, and the launcher blocks
   on any unexpected exit.
10. **A hook now sits on every step boundary.** `step-results` runs when each workflow step returns. A
    bug there stalls every workflow, and a slow grounding check on a large review adds to every pass.
    Mitigation: it reads only the files the findings name, each at most once, under the same 4 second
    deadline, and it has launcher cases with real recorded results.
11. **Scope pressure.** `artifacts.toml` could regrow the old validator family one check at a time.
    Mitigation: a check is added only for a stored fact that cannot be derived, and each kind has a
    size cap so the table itself stays small.
