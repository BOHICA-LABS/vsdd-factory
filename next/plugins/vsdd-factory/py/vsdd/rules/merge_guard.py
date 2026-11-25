"""The merge-guard rule (rules/merge-guard.md; plan rules.md 3.3).

Delivery-order gate H5: no agent merges into, pushes to, commits on or moves
the integration or release branch except through the tool. The rule reads a
tool call's text and decides in two steps, never by enumerating spellings
(S4R-rules-C1-1, S4R-rules-C1-2, S4R-rules-C2-1, S4R-rules-C2-2):

1. A segment is *git-bearing* when one of its words — after quote removal —
   is ``git`` or ends in ``/git`` (for an opaque segment, when its text,
   which carries what the segment stands for, holds ``git`` as a word).
   Anything not git-bearing is not judged.
2. A git-bearing segment is *plain* when its program is git itself, reached
   directly or through a wrapper whose option grammar the parser knows; every
   global option before the verb is in the one table of git's separate-value
   globals kept in :mod:`~vsdd.hooks.shell`; the verb is a literal word of
   git's own command list; every option of a governed verb is one the verb's
   option table knows, spelled out in full; and no word of the command holds
   a dollar sign or a backtick, except inside the value of a free-text option
   (``-m``/``--message``, ``-F``/``--file``). A plain segment is judged at
   the place git would act: the git directory when ``--git-dir`` or GIT_DIR
   names one — the last one, threaded through every ``-C`` in order — else
   the segment's own directory, which the ``-C`` options and a wrapper's
   ``--chdir`` set. A substitution inside a free-text value runs as its own
   command and is judged the same way.
3. A git-bearing segment that is not plain is not read further: it blocks
   ``opaque_git``, unless its position alone — a known directory inside a
   factory project whose checked-out branch is not guarded, and a text that
   cannot reach one — shows it harmless.
"""

from __future__ import annotations

import os
import sys

from ..hooks.shell import GIT_GLOBAL_VALUE_OPTIONS, free_text_value_words, parse

__all__: list[str] = []

#: The tool's governed commands, named by every fix line (H6 moves it when it
#: lands: decision D8).
GOVERNED = ("story deliver", "release")

_INTEGRATORS = ("merge", "rebase", "cherry-pick", "revert", "am")
_RESET_MODES = ("--soft", "--mixed", "--hard", "--merge", "--keep")
_BRANCH_MUTATORS = ("-f", "-m", "-M", "-c", "-C", "-d", "-D",
                    "--force", "--move", "--copy", "--delete")
_BRANCH_RENAMERS = ("-m", "-M", "-c", "-C", "--move", "--copy")
_VERBS = _INTEGRATORS + ("commit", "reset", "push", "update-ref", "symbolic-ref",
                         "branch", "switch", "checkout", "fetch", "pull",
                         "send-pack", "filter-branch")

#: The verbs that reach another branch from wherever they run, so an opaque
#: git-bearing segment that names one is never harmless (S4R-rules-C2-2).
#: ``send-pack`` pushes packed refs, ``fast-import`` rewrites refs from its
#: standard input, and ``filter-branch`` rewrites history in place
#: (step4-rules-fix-3-C1-4).
_FAR_VERBS = ("push", "fetch", "pull", "update-ref", "symbolic-ref", "branch",
              "switch", "checkout", "rebase", "worktree", "send-pack",
              "fast-import", "filter-branch")

#: fast-import takes its ref writes from the standard input, where the parse
#: cannot follow: the verb is never plain (step4-rules-fix-3-C1-4).
_UNREADABLE_VERBS = ("fast-import",)

#: The words of a redirect that hides where git would run: an export or a
#: bare assignment of either in an earlier segment makes every later
#: git-bearing segment unresolvable (S4R-rules-C1-2).
_GIT_ENV_NAMES = ("GIT_DIR", "GIT_WORK_TREE")

#: The options whose value is free text: a message or a file the user
#' supplies. A dollar sign or a backtick inside such a value does not blind
#: the rule; anywhere else in a git call it does (S4R-rules-C1-1).
_FREE_TEXT_OPTIONS = ("-m", "--message", "-F", "--file")

#: A ``cd``, ``pushd`` or ``popd`` word in a not-plain segment's text means
#: the directory it runs in is not the directory the parse gives
#: (step4-rules-fix-3-C1-1).
_MOVE_WORDS = ("cd", "pushd", "popd")

#: Branch-name comparisons ignore case on macOS: its file system is
#: case-insensitive, and ``git branch -f Main HEAD`` moves ``main`` there
#: (step4-rules-fix-3-C1-7).
_CASE_FOLD = sys.platform == "darwin"

#: git's own commands (step4-rules-fix-3-C1-4). A verb outside the list is an
#: alias or a spelling the parse cannot read — ``ci`` for ``commit`` — and
#: what the alias expands to is not the parse's to see, so the segment is not
#: plain.
GIT_COMMANDS = frozenset({
    "add", "am", "annotate", "apply", "archive", "bisect", "blame", "branch",
    "bugreport", "bundle", "cat-file", "check-attr", "check-ignore",
    "check-mailmap", "check-ref-format", "checkout", "checkout-index",
    "cherry", "cherry-pick", "citool", "clean", "clone", "column", "commit",
    "commit-graph", "commit-tree", "config", "count-objects", "credential",
    "credential-cache", "credential-store", "cvsexportcommit", "daemon",
    "describe", "diff", "difftool", "fast-export", "fast-import",
    "fetch", "fetch-pack", "filter-branch", "fmt-merge-msg", "for-each-ref",
    "for-each-repo", "format-patch", "fsck", "fsmonitor--daemon", "gc",
    "get-tar-commit-id", "grep", "gui", "hash-object", "http-backend",
    "http-fetch", "http-push", "imap-send", "index-pack", "init",
    "init-db", "interpret-trailers", "log", "ls-files", "ls-remote",
    "ls-tree", "mailinfo", "mailsplit", "maintenance", "merge",
    "merge-base", "merge-file", "merge-index", "merge-one-file", "merge-tree",
    "mergetool", "mktag", "mktree", "multi-pack-index", "mv", "name-rev",
    "notes", "pack-objects", "pack-redundant", "pack-refs", "patch-id",
    "prune", "prune-packed", "pull", "push", "quiltimport", "range-diff",
    "read-tree", "rebase", "receive-pack", "reflog", "remote", "repack",
    "replace", "request-pull", "rerere", "reset", "restore", "rev-list",
    "rev-parse", "revert", "rm", "send-email", "send-pack",
    "sh-i18n", "sh-setup", "shell", "shortlog", "show", "show-branch",
    "show-index", "show-ref", "sparse-checkout", "stash", "status",
    "stripspace", "submodule", "subtree", "switch", "symbolic-ref", "tag",
    "unpack-file", "unpack-objects", "update-alternatives", "update-index",
    "update-ref", "update-server-info", "upload-archive", "upload-pack",
    "var", "verify-commit", "verify-pack", "verify-tag", "whatchanged",
    "worktree", "write-tree",
})

#: The options each governed verb takes, spelled out in full: an option not
#: in the table — abbreviated (``--forc``), bundled with a governed letter
#: (``-qf``), or carrying a value attached to a branch-resetting letter
#: (``-Cmain``, ``-Bmain``, ``--force-create=main``) — makes the call not
#: plain, and the verdict goes to position. ``update-ref --stdin`` is out on
#: purpose: the ref writes ride the standard input, where the parse cannot
#: follow (step4-rules-fix-3-C1-3).
_VERB_OPTIONS = {
    "merge": frozenset({
        "-m", "--message", "-F", "--file", "--log", "--no-log", "--stat",
        "--no-stat", "-n", "--no-commit", "--commit", "-e", "--no-edit",
        "--edit", "--ff", "--no-ff", "--ff-only", "--squash", "--no-squash",
        "--abort", "--continue", "--quit", "-q", "--quiet", "-v", "--verbose",
        "--progress", "--no-progress", "-S", "--gpg-sign", "--no-gpg-sign",
        "--allow-unrelated-histories", "--autostash", "--no-autostash",
        "-X", "--strategy-option", "-s", "--strategy", "--into-name",
        "--overwrite-ignore", "--no-overwrite-ignore", "--verify",
        "--no-verify", "--cleanup", "--rerere-autoupdate",
        "--no-rerere-autoupdate", "--signoff", "--no-signoff",
    }),
    "rebase": frozenset({
        "-i", "--interactive", "-m", "--merge", "-s", "--strategy",
        "-X", "--strategy-option", "-q", "--quiet", "-v", "--verbose",
        "--stat", "--no-stat", "--autosquash", "--no-autosquash", "--onto",
        "--keep-base", "--fork-point", "--no-fork-point", "--exec", "-x",
        "--root", "--rebase-merges", "--no-rebase-merges", "--no-verify",
        "--verify", "-f", "--force", "--no-force", "-C", "--committer-date-is-author-date",
        "--ignore-date", "--ignore-whitespace", "--whitespace", "--signoff",
        "--autostash", "--no-autostash", "--empty", "--keep-empty",
        "--no-keep-empty", "--reapply-cherry-picks", "--no-reapply-cherry-picks",
        "--update-refs", "--no-update-refs", "--no-gpg-sign", "--edit-todo",
        "--show-current-patch", "-n", "--no-stat", "--no-ff",
    }),
    "cherry-pick": frozenset({
        "-e", "--edit", "--no-edit", "-x", "-m", "--mainline", "-n",
        "--no-commit", "-s", "--signoff", "-S", "--gpg-sign", "--no-gpg-sign",
        "--ff", "--allow-empty", "--allow-empty-message", "--keep-redundant-commits",
        "--strategy", "--strategy-option", "-q", "--quiet", "--cleanup",
        "--continue", "--skip", "--quit", "--abort", "--empty",
    }),
    "revert": frozenset({
        "-e", "--edit", "--no-edit", "-n", "--no-commit", "-x", "-m",
        "--mainline", "-s", "--signoff", "-S", "--gpg-sign", "--no-gpg-sign",
        "--strategy", "--strategy-option", "-q", "--quiet", "--continue",
        "--skip", "--quit", "--abort", "--cleanup", "--empty", "--ff",
    }),
    "am": frozenset({
        "-s", "--signoff", "-k", "-3", "--3way", "--no-3way", "-q", "--quiet",
        "-m", "--message-id", "-u", "--utf8", "--no-utf8", "-i",
        "--interactive", "--keep", "--keep-cr", "--no-keep-cr", "--scissors",
        "--no-scissors", "--whitespace", "--directory", "--exclude",
        "--patch-format", "--continue", "--quit", "--abort", "--retry",
        "--show-current-patch", "--empty",
    }),
    "commit": frozenset({
        "-m", "--message", "-F", "--file", "-a", "--all", "-p", "--patch",
        "-C", "--reuse-message", "-c", "--reedit-message", "--amend",
        "--no-edit", "--allow-empty", "--allow-empty-message", "--dry-run",
        "-q", "--quiet", "-v", "--verbose", "-s", "--signoff", "-e", "--edit",
        "-o", "--only", "--include", "-i", "--interactive", "-S", "--gpg-sign",
        "--no-gpg-sign", "--date", "--author", "--cleanup", "-t", "--template",
        "--status", "--no-status", "--no-post-rewrite", "-n", "--no-verify",
        "--verify", "--fixup", "--squash", "-u", "--untracked-files",
        "--pathspec-from-file", "--pathspec-file-nul", "-z", "--null",
        "--short", "--porcelain", "--long", "--ahead-behind",
        "--no-ahead-behind", "-r",
    }),
    "reset": frozenset({
        "--soft", "--mixed", "--hard", "--merge", "--keep", "-q", "--quiet",
        "-p", "--patch", "--recurse-submodules", "--no-recurse-submodules",
        "-N", "--intent-to-add", "--pathspec-from-file", "--pathspec-file-nul",
        "--refresh", "--no-refresh",
    }),
    "push": frozenset({
        "-f", "--force", "-u", "--set-upstream", "-q", "--quiet", "-v",
        "--verbose", "--all", "--mirror", "--tags", "--follow-tags", "-d",
        "--delete", "--prune", "--atomic", "--signed", "--no-signed",
        "--dry-run", "-n", "--receive-pack", "--repo", "--force-with-lease",
        "--force-if-includes", "--no-force-if-includes",
        "--recurse-submodules", "--thin", "--no-thin", "-o", "--push-option",
        "--progress", "--no-progress", "--verify", "--no-verify", "-4",
        "--ipv4", "-6", "--ipv6", "--exec", "--porcelain",
    }),
    "update-ref": frozenset({
        "-d", "--delete", "-n", "--no-deref", "-z", "--create-reflog",
        "--no-create-reflog", "-q", "--quiet",
        # --stdin is out on purpose: the ref writes ride the standard input.
    }),
    "symbolic-ref": frozenset({
        "-d", "--delete", "-q", "--quiet", "--short", "-m", "--reason",
        "--no-reason",
    }),
    "branch": frozenset({
        "-f", "--force", "-m", "--move", "-M", "-c", "--copy", "-C", "-d",
        "--delete", "-D", "--list", "-v", "--verbose", "-q", "--quiet", "-a",
        "--all", "-r", "--remotes", "-t", "--track", "--no-track",
        "--set-upstream-to", "-u", "--set-upstream", "--edit-description",
        "--contains", "--no-contains", "--merged", "--no-merged", "--sort",
        "--format", "--color", "--no-color", "-i", "--ignore-case", "--column",
        "--no-column", "--recurse-submodules", "--show-current",
        "--create-reflog", "-l", "--omit-empty", "--points-at",
    }),
    "switch": frozenset({
        "-c", "--create", "-C", "--force-create", "-d", "--detach", "--guess",
        "--no-guess", "-f", "--force", "--discard-changes", "-m", "--merge",
        "--orphan", "--overwrite-ignore", "--no-overwrite-ignore", "-t",
        "--track", "--no-track", "-q", "--quiet", "--progress",
        "--no-progress",
    }),
    "checkout": frozenset({
        "-q", "--quiet", "-f", "--force", "-m", "--merge", "--ours",
        "--theirs", "-p", "--patch", "--ignore-other-worktrees",
        "--ignore-skip-worktree-bits", "--overlay", "--no-overlay", "-b", "-B",
        "-t", "--track", "--no-track", "--orphan", "-2", "-3", "--detach",
        "--guess", "--no-guess", "-l", "--recurse-submodules",
        "--no-recurse-submodules", "--pathspec-from-file", "--pathspec-file-nul",
        "--conflict", "-d",
    }),
    "fetch": frozenset({
        "--all", "--append", "--atomic", "-f", "--force", "--multiple",
        "--dry-run", "--write-commit-graph", "-v", "--verbose", "-q", "--quiet",
        "--tags", "-t", "-p", "--prune", "-P", "--prune-tags", "-n", "--no-tags",
        "--depth", "--deepen", "--shallow-since", "--shallow-exclude",
        "--unshallow", "--update-shallow", "--refmap", "-j", "--jobs", "-u",
        "--update-head-ok", "--set-upstream", "--recurse-submodules",
        "--negotiation-tip", "--negotiate-only", "-4", "--ipv4", "-6", "--ipv6",
        "--porcelain", "--stdin", "-m",
    }),
    "pull": frozenset({
        "-v", "--verbose", "-q", "--quiet", "--ff", "--no-ff", "--ff-only",
        "-r", "--rebase", "--no-rebase", "--autostash", "--no-autostash",
        "-n", "--no-stat", "--stat", "--no-commit", "--commit", "-e", "--edit",
        "--no-edit", "--squash", "--allow-unrelated-histories", "-s",
        "--strategy", "-X", "--strategy-option", "--verify-signatures",
        "--no-verify-signatures", "-S", "--gpg-sign", "--no-gpg-sign",
        "--log", "--no-log", "--all", "--tags", "--prune", "--no-tags",
        "--dry-run", "--unshallow", "--set-upstream",
    }),
    "send-pack": frozenset({
        "--all", "--force", "-f", "--dry-run", "-n", "-q", "--quiet", "-v",
        "--verbose", "--receive-pack", "--exec", "--ipv4", "--ipv6",
        "--stateless-rpc", "--helper-status", "--thin", "--atomic",
    }),
    "filter-branch": frozenset({
        "--env-filter", "--tree-filter", "--index-filter", "--parent-filter",
        "--msg-filter", "--commit-filter", "--tag-name-filter",
        "--subdirectory-filter", "--original", "-d", "--prune-empty", "-f",
        "--force", "--use-current-ref",
    }),
}

#: The short letters of a governed verb that change, create, reset or delete
#: a branch: a bundle holding one — ``-qf`` for branch — is not read
#: letter by letter, and the call is not plain (step4-rules-fix-3-C1-3).
_GOVERNED_LETTERS = {
    "branch": frozenset("fmMcCdD"),
    "checkout": frozenset("B"),
    "switch": frozenset("C"),
    "filter-branch": frozenset("f"),
}

#: The options a value may not ride attached: ``-Cmain`` for switch,
#: ``-Bmain`` for checkout and ``--force-create=main`` reset the named branch
#: without a plain operand for the rule to read (step4-rules-fix-3-C1-3).
_NO_ATTACHED_VALUE = {
    "switch": frozenset({"-C", "--force-create"}),
    "checkout": frozenset({"-B"}),
}

_MAIN_FIX = "Start a story instead: /vsdd-factory:feature."


def check(ctx, cfg):
    """Every shell segment held to the guarded branches of its project."""
    if ctx.tool != "Bash":
        return None
    command = ctx.command
    if command is None:
        return None
    blocks = []
    poisoned = False
    for segment in command.segments:
        block = _segment(ctx, segment, poisoned)
        if block is not None and not any(
            existing.code == block.code and existing.reason == block.reason
            for existing in blocks
        ):
            blocks.append(block)
        if _exports_git_environment(segment):
            # An earlier export or assignment of GIT_DIR/GIT_WORK_TREE hides
            # where every later git call would run (S4R-rules-C1-2).
            poisoned = True
    return blocks or None


# --- one segment ---------------------------------------------------------------


def _segment(ctx, seg, poisoned):
    program = os.path.basename(seg.program) if seg.program else ""
    if program == "gh":
        if len(seg.args) >= 2 and seg.args[0] == "pr" and seg.args[1] == "merge":
            if not seg.where:
                return _block(ctx, "where_unknown", _unknown_where("gh pr merge"))
            if _guarded(ctx, seg) is None:
                return None  # a segment in no factory project is not judged
            return _block(ctx, "direct_merge", "merges a pull request with gh")
        return None
    if not _git_bearing(seg, program):
        return None
    if poisoned:
        return _block(
            ctx, "where_unknown",
            "an earlier export of GIT_DIR or GIT_WORK_TREE hides where git "
            "would run; the guarded branches are not put at risk.",
        )
    if seg.opaque:
        if program == "git" and _plain_readable(seg):
            # The only hidden text rides a free-text value: the words git
            # runs are all parsed, so the plain judgement reads them
            # (S4R-rules-C1-1).
            return _plain(ctx, seg, poisoned)
        return _not_plain(ctx, seg, poisoned)
    if program != "git":
        # Not read further (S4R-rules-C2-2): the wrapper's own grammar, an
        # unknown wrapper — position decides, not the text.
        return _not_plain(ctx, seg, poisoned)
    return _plain(ctx, seg, poisoned)


def _plain_readable(seg) -> bool:
    """Whether an opaque git segment's hiding places all sit in free-text
    values: no substitution, no expansion outside one — in the arguments, a
    redirect target or an assignment — and the plain judgement can read the
    words git runs (S4R-rules-C1-1)."""
    args = list(seg.args)
    values = free_text_value_words(args)
    for index, word in enumerate(args):
        if index in values:
            continue
        if "$" in word or "`" in word:
            return False
    for redirect in seg.redirects:
        if "$" in redirect.target or "`" in redirect.target:
            return False
    for _name, value in seg.assignments:
        if "$" in value or "`" in value:
            return False
    return True


def _git_bearing(seg, program) -> bool:
    """Whether the segment could be running git: one of its words — after
    quote removal — is ``git`` or ends in ``/git`` (S4R-rules-C1-1). A parsed
    segment is tested on its program and its arguments as whole words —
    ``grep -rn "git push" .`` is no git call — while a command string riding
    an argument (``find . -exec sh -c '… && git commit'``) is, for the words
    run as typed (step4-rules-fix-3-C1-8). An opaque segment is tested on
    its text, for it is all the parse has."""
    if program == "git":
        return True
    if seg.opaque:
        return any(word == "git" or word.endswith("/git") for word in _chunks(seg.text))
    for word in (seg.program, *seg.args):
        if word == "git" or word.endswith("/git") or os.path.basename(word) == "git":
            return True
    for word in seg.args:
        if _is_command_string(word):
            return any(part == "git" or part.endswith("/git")
                       for part in word.split())
    return False


def _is_command_string(word: str) -> bool:
    """Whether an argument is a command string of its own: it carries a shell
    joiner, so its words run as commands."""
    return "&&" in word or "||" in word or ";" in word or "|" in word or "`" in word


def _plain(ctx, seg, poisoned):
    """The plain path: git itself, literal words, judged at the place git
    would act (S4R-rules-C1-2)."""
    if _impure(seg):
        # The words git runs are not all literal. An unresolvable redirect
        # still gets the more precise verdict; every other impure call is
        # decided by position alone (S4R-rules-C1-1).
        if _git_dir_of(seg) == "" or not seg.where:
            return _unresolved(ctx)
        return _not_plain(ctx, seg, poisoned)
    words = _git_words(seg.args)
    if not words:
        return None
    verb, rest = words[0], words[1:]
    if verb not in GIT_COMMANDS:
        # An aliased or foreign verb: what it expands to is not the parse's
        # to see (step4-rules-fix-3-C1-4).
        return _not_plain(ctx, seg, poisoned)
    if verb in _UNREADABLE_VERBS:
        return _not_plain(ctx, seg, poisoned)
    if not _options_plain(verb, rest):
        return _not_plain(ctx, seg, poisoned)
    if _config_directed(seg.args, seg.assignments):
        # A -c or GIT_CONFIG_* key that redirects what a verb means — an
        # alias, a push.default, a remote's push refspec — is not plain
        # either (step4-rules-fix-3-C1-4).
        return _not_plain(ctx, seg, poisoned)
    if verb == "config" and _config_write_redirects(rest):
        return _not_plain(ctx, seg, poisoned)
    placement = _placement(ctx, seg)
    if placement is None:
        return None  # a segment in no factory project is not judged (D7)
    block = _git(ctx, seg, verb, rest, placement)
    if block is not None:
        return block
    return _value_substitutions(ctx, seg, poisoned)


def _unresolved(ctx):
    return _block(
        ctx, "where_unknown",
        "git runs where the parse cannot resolve the repository or the "
        "directory; the guarded branches are not put at risk.",
    )


def _impure(seg) -> bool:
    """Whether a word of the call holds an expansion outside a free-text
    option's value: what it expands to is part of what git runs, so the
    verdict cannot be read from the text (S4R-rules-C1-1)."""
    free_next = False
    for word in (seg.program, *seg.args):
        if free_next:
            free_next = False
            continue
        if word in _FREE_TEXT_OPTIONS:
            free_next = True
            continue
        if _free_text_attached(word):
            continue
        if "$" in word or "`" in word:
            return True
    for _name, value in seg.assignments:
        if "$" in value or "`" in value:
            return True
    return False


def _free_text_attached(word: str) -> bool:
    """An option and its free-text value as one word: ``-mx``, ``--message=x``."""
    if word.startswith("--message=") or word.startswith("--file="):
        return True
    return (
        word.startswith("-") and not word.startswith("--") and len(word) > 2
        and word[1] in "mF"
    )


def _free_value_words(args) -> set[int]:
    """The indexes of ``args`` whose word is a free-text option's value: the
    one table in :mod:`~vsdd.hooks.shell` (S4R-rules-C1-1)."""
    return free_text_value_words(args)


def _value_substitutions(ctx, seg, poisoned):
    """The substitutions inside a free-text value's word run as their own
    commands when the segment is judged: ``$(cat <<'EOF'..EOF)`` is harmless
    and ``$(git push origin develop)`` blocks like the push it is
    (S4R-rules-C1-1)."""
    values = _free_value_words(seg.args)
    for index, word in enumerate(seg.args):
        if index not in values:
            continue
        for inner in _substitution_spans(word):
            if not inner.strip():
                continue
            try:
                command = parse(inner, seg.where)
            except Exception:
                continue
            for sub_segment in command.segments:
                block = _segment(ctx, sub_segment, poisoned)
                if block is not None:
                    return block
    return None


def _substitution_spans(word: str) -> list[str]:
    """The inner texts of the ``$(...)`` and backtick spans of one word."""
    found: list[str] = []
    index = 0
    while index < len(word):
        if word.startswith("$(", index):
            end = _span_end(word, index + 1)
            found.append(word[index + 2 : end - 1] if end > index + 2 else "")
            index = end
            continue
        if word[index] == "`":
            close = word.find("`", index + 1)
            if close < 0:
                found.append(word[index + 1:])
                break
            found.append(word[index + 1 : close])
            index = close + 1
            continue
        index += 1
    return found


def _span_end(text: str, start: int) -> int:
    """The index past the ``(...)`` of a ``$(`` that opens just before
    ``start``, quotes honoured."""
    depth = 1
    index, size = start, len(text)
    while index < size:
        char = text[index]
        if char == "'":
            close = text.find("'", index + 1)
            index = size if close < 0 else close + 1
            continue
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0:
                return index + 1
        index += 1
    return size


# --- the option tables ------------------------------------------------------------


def _options_plain(verb, rest) -> bool:
    """Whether every option word of a governed verb's call is one the verb's
    table knows, spelled out in full (step4-rules-fix-3-C1-3)."""
    table = _VERB_OPTIONS.get(verb)
    if table is None:
        return True  # an ungoverned verb: its options move no branch
    governed = _GOVERNED_LETTERS.get(verb, ())
    forbidden = _NO_ATTACHED_VALUE.get(verb, ())
    for word in rest:
        if word == "--":
            break  # operands follow; the options are done
        if not word.startswith("-") or len(word) == 1:
            continue  # an operand
        if word.startswith("--"):
            name = word.split("=", 1)[0]
            if name not in table:
                return False  # abbreviated, or unknown outright
            if "=" in word and name in forbidden:
                return False  # --force-create=main
            continue
        if len(word) > 2 and word[1] in governed:
            return False  # a value attached to -C/-B: -Cmain
        if word[1] in governed:
            continue  # the plain -C, -B letter
        for letter in word[1:]:
            if not letter.isalpha():
                return False
            if letter in governed:
                return False  # a bundle holding a governed letter: -qf
            if f"-{letter}" not in table:
                return False
    return True


def _config_directed(args, assignments) -> bool:
    """Whether a ``-c``/``--config-env`` pair or a ``GIT_CONFIG_KEY_*``
    assignment sets a key that redirects what a verb does: ``alias.*``,
    ``push.*`` or ``remote.<name>.push`` (step4-rules-fix-3-C1-4)."""
    previous = ""
    for word in args:
        if previous in ("-c", "--config-env"):
            if _redirecting_key(word.partition("=")[0]):
                return True
            previous = ""
            continue
        if word.startswith("--config-env="):
            if _redirecting_key(word[len("--config-env="):].partition("=")[0]):
                return True
        previous = word
    for name, value in assignments:
        if name.startswith("GIT_CONFIG_KEY") and _redirecting_key(value.partition("=")[0]):
            return True
    return False


def _redirecting_key(key: str) -> bool:
    if not key:
        return False
    return key.startswith("alias.") or key.startswith("push.") or (
        key.endswith(".push") or key.endswith(".pushdefault")
    )


def _config_write_redirects(rest) -> bool:
    """Whether ``git config`` writes a key of the same family: the write
    plants a spelling a later call would run unseen
    (step4-rules-fix-3-C1-4)."""
    operands = [word for word in rest if not (word.startswith("-") and len(word) > 1)]
    if not operands:
        return False
    return _redirecting_key(operands[0])


# --- where git would act ---------------------------------------------------------


def _placement(ctx, seg):
    """The ``(names, at, git_directory)`` of one plain segment: the project's
    guarded names and the directory whose HEAD decides (S4R-rules-C1-2).
    ``None`` when the segment is in no factory project and names none."""
    git_dir = _git_dir_of(seg)
    if git_dir == "":
        return ("", "", True)  # given, but not resolvable: fail closed
    if git_dir is not None:
        root = _project_root(ctx, git_dir)
        if root is None:
            return None  # the repository named is in no factory project
        return (_branch_names(ctx.project_at(root)), git_dir, False)
    if not seg.where:
        return ("", "", True)  # the directory is unknowable: fail closed
    root = _project_root(ctx, seg.where)
    if root is None:
        return None
    return (_branch_names(ctx.project_at(root)), seg.where, False)


def _git(ctx, seg, verb, rest, placement):
    names, at, given = placement
    if given:
        # Fail closed: the git directory or the working directory is named so
        # badly that no branch can be read for it (S4R-rules-C1-2).
        return _block(
            ctx, "where_unknown",
            f"git {verb} runs where the parse cannot resolve the repository; "
            "the guarded branches are not put at risk.",
        )
    # The branch is judged where git itself reads HEAD: the --git-dir option
    # or a GIT_DIR assignment, never --work-tree (S4R-rules-C1-2).
    if _git_dir_of(seg) is not None:
        current = _head_of_git_dir(at) or ""
    else:
        current = ctx.branch_at(at) or ""
    positionals = _positionals(rest)
    on_guarded = any(_same_branch(current, name) for name in names)

    if verb in _INTEGRATORS:
        if on_guarded:
            return _block(ctx, "integrate_on_guarded", _on(verb, current, names))
        if verb == "rebase":
            # The last positional names the branch that gets rewritten; the
            # first is only the upstream (S4R-rules-5).
            branch_args = _rebase_branch_args(rest)
            if len(branch_args) >= 2:
                if _expanded(branch_args[-1]):
                    return _ambiguous(ctx, "rebase", names)
                if any(_same_branch(branch_args[-1], name) for name in names):
                    return _block(
                        ctx, "integrate_on_guarded",
                        f"git rebase checks out and rewrites {branch_args[-1]}, a "
                        f"guarded branch ({_of(names)}).",
                    )
        return None
    if verb == "commit":
        if on_guarded:
            return _block(ctx, "commit_on_guarded", _on(verb, current, names))
        return None
    if verb == "reset":
        if not on_guarded:
            return None
        why = _reset_moves(rest)
        if why:
            return _block(ctx, "commit_on_guarded", why)
        return None
    if verb == "push":
        return _push(ctx, names, current, positionals, rest)
    if verb == "send-pack":
        # A packed push: the refspec writes what a push writes
        # (step4-rules-fix-3-C1-4).
        return _push(ctx, names, current, positionals, rest)
    if verb == "filter-branch":
        # A rewrite in place of the branches its revision arguments name, and
        # of the checked-out branch first of all.
        if on_guarded:
            return _block(ctx, "integrate_on_guarded", _on(verb, current, names))
        for value in positionals:
            if _expanded(value):
                return _ambiguous(ctx, verb, names)
            if _is_ref(value, names):
                return _block(
                    ctx, "moves_guarded_branch",
                    f"git {verb} rewrites {_name(value)}, a guarded branch "
                    f"({_of(names)}).",
                )
        return None
    if verb in ("update-ref", "symbolic-ref"):
        for value in positionals:
            if _expanded(value):
                return _ambiguous(ctx, verb, names)
            if _is_ref(value, names):
                return _block(
                    ctx, "moves_guarded_branch",
                    f"git {verb} writes {_name(value)}, a guarded branch "
                    f"({_of(names)}).",
                )
        return None
    if verb == "branch":
        return _branch(ctx, names, current, positionals, rest)
    if verb == "switch":
        return _switch(ctx, names, rest, positionals)
    if verb == "checkout":
        return _checkout(ctx, names, rest, positionals)
    if verb in ("fetch", "pull"):
        for value in positionals[1:]:
            if ":" not in value:
                # A colon-less fetch/pull refspec writes no local branch; it
                # fills FETCH_HEAD and the remote-tracking refs (S4R-rules-C1-7).
                continue
            destination = _refspec_destination(value)
            if not destination:
                continue
            if _expanded(destination):
                return _ambiguous(ctx, verb, names)
            if _is_ref(destination, names):
                return _block(
                    ctx, "moves_guarded_branch",
                    f"the refspec writes {destination}, a guarded branch "
                    f"({_of(names)}).",
                )
        if verb == "pull" and on_guarded and len(positionals) >= 2:
            # A pull with a refspec merges what it names into the checked-out
            # branch; a bare pull passes (decision D6, S4R-rules-6).
            return _block(
                ctx, "integrate_on_guarded",
                f"git pull merges {positionals[1]} into {current}, a guarded "
                f"branch ({_of(names)}).",
            )
        return None
    return None


# --- the verbs, in detail --------------------------------------------------------


def _push(ctx, names, current, positionals, rest):
    refspecs = positionals[1:]
    for value in refspecs:
        destination = _refspec_destination(value)
        if not destination:
            continue
        if _expanded(destination):
            return _ambiguous(ctx, "push", names)
        if destination == "@" or destination.upper() == "HEAD":
            # @ is HEAD (step4-rules-fix-3-C1-3)
            if current in names:
                return _block(ctx, "push_to_guarded", _pushed(current, names))
            continue
        if _is_ref(destination, names):
            return _block(ctx, "push_to_guarded", _pushed(_name(destination), names))
    if not refspecs and current in names:
        return _block(ctx, "push_to_guarded", _pushed(current, names))
    if any(option in rest for option in ("--all", "--mirror")):
        return _block(
            ctx, "push_to_guarded",
            f"--all/--mirror pushes every branch, the guarded ones "
            f"({_of(names)}) included.",
        )
    return None


def _branch(ctx, names, current, positionals, rest):
    if not any(flag in rest for flag in _BRANCH_MUTATORS):
        return None
    for value in positionals:
        if _expanded(value):
            return _ambiguous(ctx, "branch", names)
        if _is_ref(value, names):
            return _block(
                ctx, "moves_guarded_branch",
                f"git branch changes {_name(value)}, a guarded branch "
                f"({_of(names)}).",
            )
    if any(flag in rest for flag in _BRANCH_RENAMERS) and (
        any(_same_branch(current, name) for name in names)) and len(positionals) <= 1:
        return _block(
            ctx, "moves_guarded_branch",
            f"git branch renames the current branch {current}, a guarded branch "
            f"({_of(names)}).",
        )
    return None


def _switch(ctx, names, rest, positionals):
    if not any(flag in rest for flag in ("-C", "--force-create")):
        return None
    if positionals:
        if _expanded(positionals[0]):
            return _ambiguous(ctx, "switch", names)
        if any(_same_branch(positionals[0], name) for name in names):
            return _block(
                ctx, "moves_guarded_branch",
                f"git switch -C resets {positionals[0]}, a guarded branch "
                f"({_of(names)}).",
            )
    return None


def _checkout(ctx, names, rest, positionals):
    if "-B" not in rest:
        return None
    if positionals:
        if _expanded(positionals[0]):
            return _ambiguous(ctx, "checkout", names)
        if any(_same_branch(positionals[0], name) for name in names):
            return _block(
                ctx, "moves_guarded_branch",
                f"git checkout -B resets {positionals[0]}, a guarded branch "
                f"({_of(names)}).",
            )
    return None


def _reset_moves(rest):
    """Why a ``git reset`` on a guarded branch moves it: a mode flag, or a
    revision other than ``HEAD`` — the mode searched over every argument
    before any ``--``, so ``git reset HEAD --hard`` is caught too
    (S4R-rules-22)."""
    for arg in rest:
        if arg == "--":
            break
        if arg in _RESET_MODES:
            return f"git reset {arg} moves the branch a guarded branch is checked out on."
    for arg in rest:
        if arg == "--":
            return None
        if arg.startswith("-") and len(arg) > 1:
            continue
        if arg != "HEAD":
            return f"git reset to {arg} moves the branch a guarded branch is checked out on."
        return None
    return None


def _rebase_branch_args(rest):
    """The positionals of a ``git rebase``, ``--onto``'s value aside: the last
    names the branch that gets rewritten, the first the upstream
    (S4R-rules-5)."""
    found = []
    skip = False
    for arg in rest:
        if skip:
            skip = False
            continue
        if arg == "--onto":
            skip = True  # the next word is the value, not a positional
            continue
        if arg.startswith("--onto="):
            continue
        if arg == "--":
            break
        if arg.startswith("-") and len(arg) > 1:
            continue
        if arg == "-":
            continue
        found.append(arg)
    return found


def _git_dir_of(seg):
    """The git directory this segment's git call reads HEAD from: the last
    ``--git-dir`` option before any ``--`` — git takes the last one — or a
    ``GIT_DIR`` assignment — bare, or riding a wrapper's assignments (``env
    GIT_DIR=… git …``). ``None`` when none is given, so the repository is the
    one at ``seg.where``; ``""`` when one is given but cannot be resolved,
    which blocks (S4R-rules-C1-2). A ``--work-tree`` names no repository and
    is never read here."""
    value = _git_dir_option(seg.args)
    if value is None:
        for name, assigned in seg.assignments:
            if name == "GIT_DIR":
                value = assigned
                break
    if value is None:
        return None
    return _resolve_dir(value, seg.where) or ""


def _git_dir_option(args):
    """The value of git's ``--git-dir`` option — separate or attached — or
    ``None`` when the arguments name none before any ``--``. The last one
    wins, as git's own (step4-rules-fix-3-C1-5)."""
    found = None
    previous = ""
    for arg in args:
        if previous == "--git-dir":
            found = arg
        if arg == "--":
            return found
        if arg.startswith("--git-dir="):
            found = arg[len("--git-dir="):]
        previous = arg
    return found


def _exports_git_environment(seg) -> bool:
    """Whether the segment puts GIT_DIR or GIT_WORK_TREE into the
    environment: a bare assignment before a program that is not the segment's
    own git call (``GIT_DIR=… :``, ``GIT_DIR=… sh -c …``), a spelling of
    ``export``, ``set``, ``readonly``, ``declare``, ``typeset`` or ``local``
    naming either, or an opaque segment whose text names either
    (step4-rules-fix-3-C1-6). A ``GIT_DIR=… git …`` in one segment is that
    call's own redirection, read by :func:`_git_dir_of`, not an export."""
    program = os.path.basename(seg.program) if seg.program else ""
    if _names_git_environment(seg.assignments):
        return program != "git"  # the assignment rides into this git call only
    if program in ("export", "set", "readonly", "declare", "typeset", "local"):
        if _names_git_environment([(arg, "") for arg in seg.args if "=" in arg]):
            return True
    if seg.opaque:
        chunks = _chunks(seg.text)
        return any(
            chunk.split("=", 1)[0].upper() in _GIT_ENV_NAMES for chunk in chunks
        )
    return False


def _names_git_environment(assignments) -> bool:
    return any(
        name.split("=", 1)[0] in _GIT_ENV_NAMES for name, _value in assignments
    )


def _head_of_git_dir(git_dir: str) -> str | None:
    """The branch a git directory itself has checked out: its own ``HEAD``,
    read from that directory, for a ``--git-dir`` of the main checkout or of
    a linked worktree alike (S4R-rules-C1-2). A detached HEAD yields the
    commit, which is no guarded branch's name."""
    try:
        head = (open(os.path.join(git_dir, "HEAD"), encoding="utf-8")
                .read().strip())
    except OSError:
        return None
    if not head.startswith("ref: "):
        return head or None
    ref = head[len("ref: "):]
    prefix = "refs/heads/"
    return ref[len(prefix):] if ref.startswith(prefix) else ref


def _expanded(value: str) -> bool:
    """Whether a verdict-bearing argument holds an expansion: its value at run
    time is not the parse's to know, so the verdict cannot be read from it
    (S4R-rules-C1-1)."""
    return "$" in value or "`" in value


def _ambiguous(ctx, verb, names):
    return _block(
        ctx, "opaque_git",
        f"git {verb} names an argument the parse cannot resolve, which may "
        f"be a guarded branch ({_of(names)}).",
    )


def _resolve_dir(value: str, where: str) -> str | None:
    """``value`` resolved against ``where``, symlinks followed so a link to a
    repository's git directory resolves to the repository
    (step4-rules-fix-3-C1-5); ``None`` when it names no guessable directory,
    and ``None`` again — the caller's fail-closed "" — when the name exists
    nowhere yet, for a git directory not on disk is one no branch can be
    read from."""
    if not value or value.startswith("~") or value == "-" or "$" in value or "`" in value:
        return None
    if not value.startswith("/"):
        if not where:
            return None
        value = os.path.join(where, value)
    resolved = os.path.realpath(os.path.normpath(value))
    if not os.path.exists(resolved):
        return None
    return resolved


# --- not-plain segments -------------------------------------------------------------


def _chunks(text: str) -> list[str]:
    """The words of one segment's text, quote and wrapper punctuation
    stripped, for the scans below."""
    return [word.strip("\"'`$(){};,") for word in text.split()]


def _segment_chunks(seg) -> list[str]:
    """The words a not-plain segment is scanned on. A segment whose program
    is git is scanned on its program and arguments, the free-text values
    left out — a heredoc message is message text, not command text
    (S4R-rules-C1-1); every other segment on its whole text; the assignment
    values join either way (step4-rules-fix-3-C1-8)."""
    if seg.program == "git":
        args = list(seg.args)
        values = _free_value_words(args)
        words = ["git"]
        for index, word in enumerate(args):
            if index not in values:
                words.extend(_chunks(word))
    else:
        words = _chunks(seg.text or "")
    for _name, value in seg.assignments:
        words.extend(_chunks(value))
    return words


def _not_plain(ctx, seg, poisoned):
    """A git-bearing segment the rule cannot read further (S4R-rules-C2-2):
    it blocks ``opaque_git`` unless position alone shows it harmless
    (S4R-rules-C1-1)."""
    chunks = _segment_chunks(seg)
    if _is_gh_pr_merge(chunks):
        return _block(
            ctx, "opaque_git",
            "the command runs gh pr merge where the parse cannot see ("
            + _of(_guarded_known(ctx)) + ").",
        )
    block = _harmless_or_block(ctx, seg, chunks)
    if block is not None:
        return block
    return _value_substitutions(ctx, seg, poisoned)


def _harmless_or_block(ctx, seg, chunks=None):
    """Position alone: a not-plain git call in a known directory inside a
    factory project, on a branch that is not guarded, whose text names no
    guarded branch, no redirection, no ``cd`` and no far-reaching verb, and
    whose substitutions run nothing that reaches one, cannot touch one
    (S4R-rules-C2-2). A segment in no factory project blocks when its text
    itself names a way to reach one: a ``-C``, a ``--git-dir``, a
    ``--work-tree``, a GIT_DIR/GIT_WORK_TREE spelling, a ``cd`` or a far verb
    (S4R-rules-C1-2). Every other not-plain git call blocks."""
    if not seg.where:
        return _block(ctx, "opaque_git", _unplacable())
    if chunks is None:
        chunks = _segment_chunks(seg)
    root = _project_root(ctx, seg.where)
    if root is None:
        # Not this project's business — unless the text itself carries the
        # reach into one (S4R-rules-C1-2).
        if any(_redirects(chunk) or chunk in _MOVE_WORDS
               or chunk.lower() in _FAR_VERBS for chunk in chunks):
            return _block(ctx, "opaque_git", _unplacable())
        return None  # a segment in no factory project is not judged (D7)
    names = _branch_names(ctx.project_at(root))
    current = ctx.branch_at(seg.where) or ""
    if any(_same_branch(current, name) for name in names):
        return _block(ctx, "opaque_git", _unseen(names))
    for chunk in chunks:
        if chunk.lower() in _FAR_VERBS:
            return _block(ctx, "opaque_git", _unseen(names))
        if chunk in _MOVE_WORDS:
            # A cd word in the segment's text: the directory it runs in is
            # not the directory the parse gives (step4-rules-fix-3-C1-1).
            return _block(ctx, "opaque_git", _unseen(names))
        if _redirects(chunk):
            return _block(ctx, "opaque_git", _unseen(names))
        for name in names:
            if _chunk_names(chunk, name):
                return _block(ctx, "opaque_git", _unseen(names))
    return None


def _chunk_names(chunk: str, name: str) -> bool:
    """Whether one scanned word names a guarded branch — by letter-case too
    on macOS, whose file system answers ``Main`` for ``main``
    (step4-rules-fix-3-C1-7)."""
    if _same_branch(chunk, name):
        return True
    if chunk.endswith(f"/{name}") or f"{name}:" in chunk:
        return True
    if _CASE_FOLD:
        lowered = chunk.lower()
        tail = f"/{name.lower()}"
        return lowered.endswith(tail) or f"{name.lower()}:" in lowered
    return False


def _redirects(chunk: str) -> bool:
    """Whether one word of an unreadable call redirects where git would run:
    a ``-C``, a ``--git-dir``/``--work-tree`` — separate or attached — or a
    GIT_DIR/GIT_WORK_TREE spelling (S4R-rules-C1-2)."""
    head = chunk.split("=", 1)[0].upper()
    if head in _GIT_ENV_NAMES:
        return True
    if chunk in ("-C", "--git-dir", "--work-tree"):
        return True
    return chunk.startswith(("-C/", "--git-dir=", "--work-tree=")) or (
        chunk.startswith("-C") and len(chunk) > 2
    )


def _same_branch(left: str, right: str) -> bool:
    """Whether two branch names are the branch: literally everywhere, and
    letter-case aside on macOS, whose file system answers ``Main`` for
    ``main`` (step4-rules-fix-3-C1-7)."""
    if not left or not right:
        return False
    return left == right or (_CASE_FOLD and left.casefold() == right.casefold())


def _unseen(names) -> str:
    return (
        "the command runs git where the parse cannot see; the guarded "
        "branches are not put at risk (" + _of(names) + ")."
    )


def _unplacable() -> str:
    return (
        "the command runs git in a directory the parse cannot resolve; the "
        "guarded branches are not put at risk."
    )


def _is_gh_pr_merge(chunks):
    for index, word in enumerate(chunks[:-2]):
        if word == "gh" and chunks[index + 1] == "pr" and chunks[index + 2] == "merge":
            return True
    return False


def _guarded_known(ctx):
    """The guarded branch names of the session's project, for a message."""
    if ctx.root:
        from ..project import find_project_root

        from ..config import load
        try:
            config = load(find_project_root(ctx.root, home=ctx.env.get("HOME"))
                          / ".factory" / "factory.toml")
            data = getattr(config, "data", config)
            branches = data.get("branches", {}) if isinstance(data, dict) else {}
            return (
                branches.get("integration", "develop"),
                branches.get("release", "main"),
            )
        except Exception:
            return ("develop", "main")
    return ("develop", "main")


# --- helpers -----------------------------------------------------------------------


def _project_root(ctx, where):
    """The project root ``where`` itself sits in, or ``None``: found from
    ``where`` alone, never falling back to the session's (S4R-rules-4)."""
    if not where:
        return None
    from ..project import find_project_root

    try:
        return find_project_root(where, home=ctx.env.get("HOME") or None)
    except Exception:
        return None


def _guarded(ctx, seg):
    """The guarded branch names of the segment's own project — the project
    ``seg.where`` sits in, read with that project's configuration (decision
    D7, S4R-rules-4) — or ``None`` when the segment is in no factory
    project."""
    root = _project_root(ctx, seg.where)
    if root is None:
        return None
    return _branch_names(ctx.project_at(root))


def _branch_names(project):
    """The guarded branch names of one project, or ``None`` without one."""
    if project is None:
        return None
    data = getattr(project.config, "data", project.config)
    branches = data.get("branches", {}) if isinstance(data, dict) else {}
    integration = branches.get("integration", "develop")
    release = branches.get("release", "main")
    return (integration, release)


def _git_words(args):
    """The verb and its arguments, git's global options skipped: those that
    take a separate value consume it (the one table, S4R-rules-C2-1)."""
    index = 0
    count = len(args)
    while index < count:
        arg = args[index]
        if arg in GIT_GLOBAL_VALUE_OPTIONS and index + 1 < count:
            index += 2  # the option and its separate value
            continue
        if arg.startswith("-") and len(arg) > 1:
            index += 1  # a bare global, or one with its value attached
            continue
        return args[index:]
    return []


def _positionals(rest):
    """The non-option arguments; operands after a ``--`` are operands still,
    for ``git branch -f -- main HEAD`` moves ``main`` as surely
    (step4-rules-fix-3-C1-3)."""
    found = []
    end_of_options = False
    for arg in rest:
        if not end_of_options and arg == "--":
            end_of_options = True
            continue
        if not end_of_options and arg.startswith("-") and len(arg) > 1:
            continue
        if not end_of_options and arg == "-":
            continue
        found.append(arg)
    return found


def _refspec_destination(value: str) -> str | None:
    """The destination of one push/fetch refspec: ``b``, ``src:b``,
    ``+src:b``, ``:b``, ``refs/heads/b``, ``HEAD``."""
    text = value[1:] if value.startswith("+") else value
    if ":" in text:
        destination = text.rsplit(":", 1)[1]
    else:
        destination = text
    destination = destination.strip()
    if not destination or destination.startswith("^"):
        return None
    return destination


def _is_ref(value: str, names) -> bool:
    return any(
        _same_branch(value, name) or value == f"refs/heads/{name}"
        or (_CASE_FOLD and value.casefold() == f"refs/heads/{name}".casefold())
        for name in names
    )


def _name(value: str) -> str:
    suffix = "refs/heads/"
    return value[len(suffix):] if value.startswith(suffix) else value


def _on(verb: str, current, names) -> str:
    return (
        f"git {verb} on {current}, a guarded branch ({_of(names)}), changes it "
        "outside the tool."
    )


def _pushed(branch, names) -> str:
    return f"git push writes {branch}, a guarded branch ({_of(names)})."


def _unknown_where(verb: str) -> str:
    return (
        f"git {verb} runs in a directory the parse cannot resolve; the guarded "
        "branches are not put at risk."
    )


def _of(names) -> str:
    return "guarded: " + ", ".join(names)


def _block(ctx, code: str, reason: str):
    from ..hooks.model import Block

    fix = (
        "Run the git command by itself, plainly, from the story worktree; or "
        f"run the tool instead: vsdd {GOVERNED[0]} or vsdd {GOVERNED[1]}."
    )
    try:
        if ctx.row.name == "main":
            fix = f"{fix} {_MAIN_FIX}"
    except Exception:
        pass  # an unreadable wall row is not the fix line's business
    return Block(code, reason, fix)
