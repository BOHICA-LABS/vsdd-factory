"""The hook runner: dispatch, context, output, failure behaviour (package H1).

Modules:

- :mod:`vsdd.hooks.model` — the outcome types, the rule declaration
  (:class:`RuleEntry`, HD1) and :class:`Ctx`, the per-call context;
- :mod:`vsdd.hooks.registry` — the rule table, event rows, tool atoms, and the
  ``hooks.json`` generator (B4b);
- :mod:`vsdd.hooks.main` — the entry: read one call, select, run under the
  deadline, emit;
- :mod:`vsdd.hooks.emit` — the only writer of a hook's stdout, stderr and exit
  code;
- :mod:`vsdd.hooks.caller` — the caller marker and the command rewrite
  ([[record:caller-marker]]);
- :mod:`vsdd.hooks.shell` — the POSIX sh splitter for Bash commands;
- :mod:`vsdd.hooks.gitfs` — git facts from files and the one-subprocess helper;
- :mod:`vsdd.hooks.policy` — the shipped policy tables as the rules read them;
- :mod:`vsdd.hooks.paths` — the path classes of a target, behind ``ctx.classes``;
- :mod:`vsdd.hooks.liveness` — keeping the run lock fresh and releasing it when
  a session ends.

Standard library only (hooks.md 2.4; the 3.11 floor is ``tomllib``'s).
"""
