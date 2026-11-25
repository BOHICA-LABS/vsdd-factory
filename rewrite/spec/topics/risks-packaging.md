---
kind: topic
id: risks-packaging
sources: [packaging.md#7]
---

# Risks of the packaging design

The packaging design names thirteen risks. Each is stated here as the design states it; what mitigates a risk is
named in the risk itself, and the thing that carries the mitigation is linked.

1. **Silent marketplace install failure.** It happened five times in the release-candidate line, and the validator
   passed each time. The mitigation is `install-check` in the marketplace repository
   ([[packaging:install-check]]), which rests on N3 and N4 ([[packaging:packaging-assumptions]]).
2. **The floor is one version.** Everything was verified on 2.1.331 only, and workflows are a young platform feature.
   An upstream change can break the plugin with no change here. The weekly conformance tier against the newest
   Claude Code is the early warning; it costs about a dollar a week.
3. **`vsdd` on `PATH` inside workflow agents is documented, not tested** (N1). The whole tool design assumes a bare
   command name. The fallback works but puts a long cache path into every agent file.
4. **End-to-end tests are model-driven.** As a release gate they can fail for no reason, and they cost money. The
   override exists and is visible; the temptation will be to use it by habit.
5. **The harness is a second implementation of the platform's rules.** If it is wrong, a workflow passes here and
   fails for real. The conformance tier pins each rule to the real platform, and the smoke tier always runs one real
   workflow.
6. **The shared rc.25 cache directory.** When the first project updates, Claude Code may mark the old directory for
   deletion in fourteen days while seven projects still use it. A project that loses it fetches the current release
   at its next session and lands on 1.0 with old state. That is safe only if the inert-on-old-state property (H4,
   S1) is built. This is the most important dependency this design has on the others ([[topic:hooks-inert]]).
7. **Residue in migrated projects.** `prism` and `pregolya` each have a `CLAUDE.md` of about 600 lines of old process
   rules, loaded into every agent there; the cutover tool reports this and does not fix it
   ([[packaging:cutover-projects]]). Until a person edits those files, 1.0's agents in those projects are told to
   follow rules that no longer exist.
8. **Bundled servers.** All eight projects already configure the same servers themselves, so the plugin's copies are
   dropped as duplicates there or run alongside them, and tool names differ between machines
   ([[topic:research-and-browser-tools]]). `npx` servers need Node, which Claude Code no longer brings with it. The
   behaviour with no key set is unknown (N6).
9. **The promotion commit deletes about 200,000 lines and 190 MB.** It is mechanical and reversible only through git.
   The archive tag and pinning the marketplace first (X1, X2) are what make it safe; neither may be skipped
   ([[packaging:cutover-steps]]).
10. **`uv` is a hard prerequisite.** If Claude Code starts with a `PATH` that lacks it, the launcher's fallback
    locations cover the common installs. Beyond that every tool call is blocked with the install message, which is
    the intended loud failure but will be someone's first experience.
11. **Builders will read the old tree** to port content and may carry its vocabulary across. L-DEAD catches the
    listed terms only; reviewers need to watch for old process ideas in new words ([[table:lint-checks]]).
12. **A plugin with a `bin/` folder cannot be installed in claude.ai or Cowork.** This is accepted: 1.0 targets
    Claude Code on macOS and Linux, and the hooks and workflows would not run there anyway.
13. **Required checks.** If branch protection names the old jobs, the cutover pull request cannot merge until a
    person changes the list. It is a one-minute step that is easy to forget and blocks everything behind it
    ([[packaging:ci]]).
