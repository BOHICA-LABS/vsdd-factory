---
kind: topic
id: caller-identity
sources: [hooks.md#2.11.10, BLUEPRINT.md#4.9, BLUEPRINT.md#4.13, BLUEPRINT.md#4.1]
---

# Caller identity

A `vsdd` process cannot see the hook payload, so it cannot know which session or which agent called
it. Before the runner lets a `vsdd` Bash call through, it does two things. It writes a caller marker,
`.factory/local/hooks/calls/<key>.json`, holding the payload's `session_id`, `agent_id`, `agent_type`
and `cwd` and the time; the key is random and used once. And it returns `updatedInput` for the call,
with the command rewritten to begin `VSDD_CALLER=<key> `, so the tool can find its own marker and no
other. A caller cannot forge it: an agent's own `VSDD_*` assignment is blocked, and markers are in a
class only code writes.

One marker per session is not enough, because a `vsdd` process has no way to pick out its own session's
marker: with two
sessions in one project, which the state design supports, the survey of a session whose hooks were off
passes on the other session's marker, and the lock could be recorded under the wrong session and
released when that other session ended. With a marker per call:

- `vsdd survey` refuses to start a run when its own call has no marker, so a workflow cannot run with
  the hooks disabled at the platform level, with the plugin's hooks switched off, or with
  `VSDD_FACTORY_HOOKS=off`, whatever another session is doing.
- The lock records the session that actually called.
- `vsdd spike open` learns the runner's `agent_id`, so parallel runners cannot be swapped.
- `vsdd env start` records which agent started a product, so it can be stopped when that agent stops.
- `vsdd doctor`, run through the Bash tool, finds its marker and reports the hooks as live; run in a
  terminal it says the hooks cannot be checked from there.

A marker is deleted when the tool reads it, and unread markers older than ten minutes are pruned. The
rewrite is one more thing `auto-allow` and `tool_only` must accept: the runner adds the prefix after
every rule has judged the command as the caller typed it.

## What a plain `vsdd` call is

Which Bash calls the runner may mark and approve — "a command of exactly one segment, not opaque,
with no redirect and no leading assignment, whose program's last path part is `vsdd`" — is
decided on the raw command text first. Four review rounds each found a shell construct (a quote
nested in `${...}`, bash and zsh `$[..]`, ANSI-C `$'...'`, bash's `${name@P}`) that read as one
plain `vsdd` call while running something else, so a deny-list over shell syntax cannot be shown
complete across `sh`, `bash`, `dash` and `zsh`: the parser's `not opaque` is never by itself a
reason to approve. The whole text must instead match, whole:

- optional leading and trailing spaces or tabs;
- the program, which is `vsdd` or a path of safe characters that ends in `/vsdd` and holds no `=` (a
  first word holding one is an assignment, and the shell runs the next word, not the path) and begins
  with a letter, a digit, `_`, `.` or `/`: bash, macOS `/bin/sh` and zsh read a first word beginning
  with `%` as a job spec and run the `fg` builtin instead of the path, and the `/bin/sh` interpreter
  of a `#!/bin/sh` script invoked through a word beginning with `-` or `+` reads that word as
  options, so such a call is approved yet can never run `vsdd`; the path may
  be the bare `/vsdd`, an ordinary absolute path in every one of the shells,
  exactly like `//vsdd` (step3-hook-runner-fix-6-C1-1);
- then zero or more arguments, each introduced by one or more spaces or tabs.

An argument is a bare word of safe characters that does not start with `=`, or a bare-word prefix
that is empty or ends with `=` followed by one single-quoted string, or one single-quoted string
alone. The safe characters are ASCII letters, digits and `_ @ % + = : , . / -`; a bare word does
not start with a tilde, which is not safe. A single-quoted string holds no single quote and no
character below U+0020 or U+007F, and is followed by a space, a tab or the end of the text. Nothing
else may appear anywhere: no double quote, dollar sign, backtick, backslash, semicolon, ampersand,
pipe, angle bracket, parenthesis, brace, square bracket, star, question mark, tilde, hash,
exclamation mark, newline or carriage return — each can start a construct (a joiner, an expansion,
a redirect, a comment, a glob, a quoted span) whose reading differs across the shells or runs more
than the call. A call whose text fails the grammar passes unmarked and undecided, exactly like any
other command that runs `vsdd`; the parsed shape above stays as the second gate.

## Proven, and what follows if a later release changes it

Both things this rests on are proven: `updatedInput` is honoured for a Bash call inside a workflow step,
and calls inside a step carry the main session's `session_id` ([[topic:platform-proven]], items 27 and
28; [[topic:platform-proven-2]], item 38). The two fallbacks below are not built; they are kept as what
would replace each if a later release changed it. Neither fallback uses the payload's `cwd`: a step that is not isolated runs in the
project root, so `cwd` is the same for every story running at once ([[topic:rule-model]]).

If the first fails, the key cannot be handed to the call, so the tool finds its marker by what the
call is. The runner names the marker by a hash of the tokens of the `vsdd` segment after the program
name; the tool hashes its own arguments the same way and takes the oldest unread marker of that name,
because calls start in the order their hooks ran. Pipeline commands name their run, story or step, so
two calls made at once differ. This needs no process walk, so item 29 decides nothing here.

For the second, neither liveness nor the lock's safety rests on the session id alone: the run lock
is kept alive by calls that name the run or touch its worktrees ([[topic:run-lock]]). What is lost is
convenience: a lock left by a stopped run is then freed only when it goes stale or by
`vsdd lock release --force`.

The marker store and the spike admission store are machine records ([[record:caller-marker]],
[[record:spike-admission]]) in the state layout; the liveness contract is in [[topic:run-lock]].
