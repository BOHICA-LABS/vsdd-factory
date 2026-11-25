"""The caller marker and the command rewrite (plan 7, test_caller.py)."""

from __future__ import annotations

import json
import os
import random
import stat

import pytest

from tests.hooks import runner_helpers as helpers
from tests.hooks.test_runner import base_env, bash_call, read_events
from vsdd.hooks import caller
from vsdd.hooks.emit import DecisionlessRewrite
from vsdd.hooks.model import RuleEntry
from vsdd.schema import load_registry, validate

NEXT = helpers.NEXT
REGISTRY = load_registry(NEXT / "plugins" / "vsdd-factory" / "schemas")

VSDD_CALL = "vsdd survey"


def rule_on(pair, **overrides) -> RuleEntry:
    """One fixture-rule row selecting `pair`, with overrides."""
    fields = dict(
        id="probe",
        on=(pair,),
        kind="gate",
        fail="closed",
        module="tests.hooks.fixture_rules.rewriting",
    )
    fields.update(overrides)
    return RuleEntry(**fields)


def run_gate(project, command=VSDD_CALL, env_extra=None):
    env = base_env(project, env_extra or {})
    return helpers.run_hook(bash_call(project, command), "gate", env=env)


def test_marker_and_prefix(tmp_path):
    project = helpers.make_project(tmp_path)
    code, out, err = run_gate(project, env_extra={"VSDD_FACTORY_HOOKS_DEBUG": "1"})
    markers = helpers.root_markers(project)
    assert code == 0 and len(markers) == 1

    marker = json.loads(markers[0].read_text())
    errors = validate(marker, REGISTRY["caller-marker.schema.json"], REGISTRY)
    assert not errors, errors  # a schema-valid file (HD13)

    # A new key per call, and the marker is the file's name:
    assert marker["key"] == markers[0].stem
    assert marker["session_id"] == "sess_9f83ab"
    assert marker["agent_id"] == "" and marker["agent_type"] == ""  # the main session
    assert marker["cwd"] == str(project)
    assert marker["ts"].endswith("Z") and "." in marker["ts"]  # the event log's form

    # The rewrite carries the whole tool_input with the prefix:
    rendered = json.loads(out)["hookSpecificOutput"]
    assert rendered["updatedInput"]["command"].startswith(
        f"VSDD_CALLER={marker['key']} {VSDD_CALL}"
    )


def test_new_key_per_call_and_other_fields_kept(tmp_path):
    project = helpers.make_project(tmp_path)
    code, out, err = run_gate(project)
    first = json.loads(out)["hookSpecificOutput"]["updatedInput"]["command"].split(" ")[0]
    assert first.startswith("VSDD_CALLER=")
    first_key = first[len("VSDD_CALLER="):]

    code, out, err = run_gate(project)
    markers = helpers.root_markers(project)
    assert len(markers) == 2
    keys = {json.loads(marker.read_text())["key"] for marker in markers}
    assert len(keys) == 2  # a new key per call
    assert first_key in keys

    # Other input fields are kept whole (only the command gains the prefix):
    payload = bash_call(project, VSDD_CALL, tool_input={"command": VSDD_CALL, "timeout": 30})
    code, out, err = helpers.run_hook(payload, "gate", env=base_env(project))
    rendered = json.loads(out)["hookSpecificOutput"]
    third_command = rendered["updatedInput"]["command"]
    key_in_command = third_command.split(" ")[0][len("VSDD_CALLER="):]
    assert third_command == f"VSDD_CALLER={key_in_command} {VSDD_CALL}"
    markers = helpers.root_markers(project)
    assert any(json.loads(marker.read_text())["key"] == key_in_command
               for marker in markers)
    assert rendered["updatedInput"]["timeout"] == 30


def test_minus_c_honoured(tmp_path):
    project = helpers.make_project(tmp_path)
    nested = project / "nested"
    nested.mkdir()
    # The -C directory picks the project the same way the tool must (plan 3.5):
    env = base_env(project)
    payload = bash_call(project, f"vsdd -C {nested} survey")
    code, out, err = helpers.run_hook(payload, "gate", env=env)
    assert code == 0
    marker = json.loads(helpers.root_markers(project)[0].read_text())
    assert marker["cwd"] == str(project)  # the payload cwd, not -C


def test_decision_pairing(tmp_path):
    """HD7 (P3COV-07, PR3-3): allow with a Permit, agent_id, a
    StructuredOutput call or an automatic mode; otherwise ask."""
    project = helpers.make_project(tmp_path)

    decision_of = lambda out: json.loads(out)["hookSpecificOutput"]

    # The main session, default mode: ask, the reason prefixed.
    code, out, err = run_gate(project)
    rendered = decision_of(out)
    assert rendered["permissionDecision"] == "ask"
    assert rendered["permissionDecisionReason"].startswith("vsdd-factory hook:")

    # An agent: allow.
    agent = bash_call(project, VSDD_CALL)
    agent["agent_id"] = "agent-3"
    agent["agent_type"] = "vsdd-factory:implementer"
    code, out, err = helpers.run_hook(agent, "gate", env=base_env(project))
    assert decision_of(out)["permissionDecision"] == "allow"

    # Automatic mode: allow.
    auto = bash_call(project, VSDD_CALL, permission_mode="auto")
    code, out, err = helpers.run_hook(auto, "gate", env=base_env(project))
    assert decision_of(out)["permissionDecision"] == "allow"

    bypass = bash_call(project, VSDD_CALL, permission_mode="bypassPermissions")
    code, out, err = helpers.run_hook(bypass, "gate", env=base_env(project))
    assert decision_of(out)["permissionDecision"] == "allow"

    # A StructuredOutput call: allow even in the default mode.
    structured = helpers.payload("PreToolUse/output-structured.json")
    structured["permission_mode"] = "default"
    structured["tool_input"] = {"command": "whatever", "result": {}}
    # StructuredOutput is not a Bash call: no rewrite, no decision.
    code, out, err = helpers.run_hook(structured, "gate", env=base_env(project))
    assert code == 0 and out == ""

    # HD8 (C2-1, step3-hook-runner-fix-3-C1-1): a command holding a
    # substitution — or hiding one in brackets, or hiding a second command in
    # bracket text or after a carriage return — is opaque or several segments,
    # so an agent's vsdd call shaped like this is no vsdd call: no marker, no
    # decision, no rewrite. It approves nothing but the command as typed.
    marked = len(helpers.root_markers(project))  # the calls above wrote theirs
    for command in (
        "vsdd survey $(id)$((1))",
        "vsdd survey $((id) )",
        "vsdd survey $((id)|(id))",
        "vsdd survey [;id;]",
        "vsdd survey [$(id)]",
        "vsdd survey $[$(id)]",
        "vsdd survey \r#; id",
    ):
        agent = bash_call(project, command)
        agent["agent_id"] = "agent-5"
        agent["agent_type"] = "vsdd-factory:implementer"
        code, out, err = helpers.run_hook(agent, "gate", env=base_env(project))
        assert code == 0 and out == "", (command, out, err)
    assert len(helpers.root_markers(project)) == marked  # none of these marked


def _parse_call(command):
    from vsdd.hooks.shell import parse

    return parse(command, "/base")


# ORCH-2: approval is decided on the raw command text first. The parsed HD8
# shape is only the second gate; the parser's "not opaque" is never by itself
# a reason to approve.

ALLOWED_CALLS = (
    "vsdd",
    "vsdd survey",
    "  vsdd survey  ",
    "vsdd\t survey",
    "vsdd survey --json --dry-run",
    "vsdd -C /repo/sub survey",
    "./vsdd survey",
    "/usr/local/bin/vsdd survey",
    # The bare "/vsdd" is a plain absolute path, exactly like "//vsdd"
    # (step3-hook-runner-fix-6-C1-1).
    "/vsdd survey",
    "vsdd survey --name='two words'",
    "vsdd FOO='bar baz' survey",
    "vsdd survey 'aé'",
    "vsdd '/path with spaces/ünïcode' survey",
    "vsdd survey a+b=c:d,e.f/g-h_i%@x",
)

#: One case per excluded character, plus the grammar's edges.
REFUSED_CALLS = (
    'vsdd survey "x"',
    "vsdd survey $(id)",
    "vsdd survey `id`",
    "vsdd survey a\\b",
    "vsdd survey; id",
    "vsdd survey & id",
    "vsdd survey | id",
    "vsdd survey >out",
    "vsdd survey <in",
    "vsdd survey (x)",
    "vsdd survey a)b",
    "vsdd survey {x}",
    "vsdd survey a}b",
    "vsdd survey a[x]",
    "vsdd survey a]b",
    "vsdd survey a*b",
    "vsdd survey a?b",
    "vsdd survey ~x",
    "vsdd survey #x",
    "vsdd survey !x",
    "vsdd survey\n",
    "vsdd survey\r",
    "vsdd survey =x",
    "vsdd survey 'a",
    "vsdd survey 'a'b",
    # A quoted span holds no single quote: junk that carries one cannot hide
    # between an outer pair of quotes (ORCH-2).
    "vsdd survey 'a'b'",
    "vsdd survey 'a''b'",
    "vsdd survey 'x'; id; 'y'",
    "vsdd survey ''$(id)''",
    # A quoted span holds no character below U+0020 and no U+007F (ORCH-2): a
    # tab, a newline, a carriage return, a NUL, an escape character and DEL.
    "vsdd survey 'a\tb'",
    "vsdd survey 'a\nb'",
    "vsdd survey 'a\rb'",
    "vsdd survey 'a\x00b'",
    "vsdd survey 'a\x1bb'",
    "vsdd survey 'a\x7fb'",
    # A quoted span may follow a bare word only when that word ends with "="
    # (ORCH-2): a bare letter glued to a quoted string is refused.
    "vsdd survey a'b'",
    "",
    "sh vsdd survey",
    "vsddx survey",
    "FOO=1 vsdd survey",
    # The program's path holds no "=": a first word holding one is an
    # assignment, and the shell runs the next word, not the path
    # (step3-hook-runner-fix-4-C1-1).
    "FOO=/vsdd id",
    "a=b/vsdd survey",
    "vsdd/survey x",
    "vsdd survey = 'a b'",
    # The program's path begins with a letter, a digit, "_", "." or "/"
    # (step3-hook-runner-fix-4-C2-1): bash, macOS /bin/sh and zsh read a first
    # word beginning with "%" as a job spec and run the fg builtin, not the
    # path, and the interpreter of a "#!/bin/sh" script invoked through a word
    # beginning with "-" or "+" reads that word as options, so such a call is
    # approved and can never work.
    "%/vsdd survey",
    "%1/vsdd x",
    "-x/vsdd survey",
    "+/vsdd survey",
    # Sole-cause rows for the argument- and program-position rules
    # (step3-hook-runner-fix-5-C2-3): a dollar sign or an opening parenthesis
    # is no safe character, a quoted prefix may not start with "=" (zsh's
    # equals expansion reads ='a' as the path of the command a), a carriage
    # return or a line feed is neither a separator nor a leading blank, and a
    # path holds no "=" in any position. The parser refuses some of these
    # too; the regex-alone test below holds them either way.
    "vsdd survey $x",
    "vsdd survey a$b",
    "vsdd survey a(b",
    "vsdd survey ='a'",
    "vsdd survey\rx",
    "vsdd survey\nx",
    "\rvsdd survey",
    "sub=/vsdd survey",
    "=/vsdd survey",
    "=x/vsdd survey",
)


def test_allow_list_accepts_plain_calls():
    """ORCH-2: the grammar of caller-identity accepts the plain forms — bare
    words of safe characters, quoted strings holding blanks and non-ASCII, a
    `name='value'` prefix — and leading or trailing blanks and tabs."""
    for command in ALLOWED_CALLS:
        assert caller.is_vsdd_call(_parse_call(command)), command


def test_allow_list_refuses_every_excluded_character():
    """ORCH-2: any command whose text holds an excluded character, or breaks
    the grammar's edges, passes unmarked whatever the parse says — the
    parser's "not opaque" alone is never a reason to approve."""
    for command in REFUSED_CALLS:
        assert not caller.is_vsdd_call(_parse_call(command)), command


#: Every bypass command the ledger records for this unit, by finding id.
LEDGER_BYPASS_FORMS = {
    "S3R-hook-runner-C1-1": (
        "vsdd survey $(id)$((1))",
        "vsdd survey `id`$((1))",
        'vsdd survey "$(id)$((1))"',
        "X=$(id)$((1)) ls",
        "echo > $(id)$((1))",
    ),
    "S3R-hook-runner-C2-1": (
        "vsdd survey $(echo pwned > /dev/null)$((1))",
        "vsdd survey $((echo pwned) )",
        "vsdd survey $((id)|(id))",
        "vsdd survey \"${x:-\"'\"}\"$(id)''",
        "vsdd survey \"${x:-'\"'}\"$(id)''",
        "vsdd survey \"${(e):-\\$(id)}\"",
        "vsdd survey \"${x:=\\$(id)}${x@P}\"",
        "vsdd survey \"${x:=a[\\$(id)]}\" ${!x}",
        "vsdd survey \"${x:=a[\\$(id)]}\" $((x))",
    ),
    "step3-hook-runner-fix-3-C1-1": (
        "vsdd survey [ ; id ; ]",
        "vsdd survey [;id;]",
        "vsdd survey [ && id ]",
        "vsdd survey a[b|id]",
        "vsdd survey [$(id)]",
        "vsdd survey [`id`]",
        "vsdd survey $[$(id)]",
        "vsdd survey $['$(id)']",
        "vsdd survey \"$[a[\\$(id)]]\"",
    ),
    "step3-hook-runner-fix-3-C1-2": (
        "vsdd survey \r#; id",
        "vsdd survey \r#; rm -rf /tmp/x",
    ),
    "step3-hook-runner-fix-3-C1-3": (
        "10>/dev/null git push --force origin main",
        "git 10>/dev/null push",
    ),
    "step3-hook-runner-fix-3-C2-1": (
        "vsdd survey $'\\'' ; id #'",
        "vsdd survey $'\\'';id;'x'",
    ),
    "step3-hook-runner-fix-3-C2-2": (
        "vsdd survey '$(id)' ${BASH_EXECUTION_STRING@P}",
        "vsdd survey ${x@P}",
        "vsdd survey ${!x}",
    ),
}


def test_allow_list_refuses_every_ledger_bypass_form():
    """ORCH-2: every bypass command recorded in the findings ledger for this
    unit — one finding id at a time — is no vsdd call: the raw text is refused
    whoever parses it."""
    forms = [form for forms in LEDGER_BYPASS_FORMS.values() for form in forms]
    assert len(forms) >= 20  # the ledger's record stays covered end to end
    for command in forms:
        assert not caller.is_vsdd_call(_parse_call(command)), command


def test_allow_list_regex_refuses_the_tables_on_its_own():
    """ORCH-2: the regular expression alone refuses every table entry, so no
    hole in the text grammar can hide behind the parsed shape — and the
    assignment-shaped programs of step3-hook-runner-fix-4-C1-1 with it, whose
    first word the shell reads as an assignment."""
    for command in REFUSED_CALLS:
        assert caller._CALL_TEXT_RE.fullmatch(command) is None, command
    for forms in LEDGER_BYPASS_FORMS.values():
        for command in forms:
            assert caller._CALL_TEXT_RE.fullmatch(command) is None, command


def test_refused_bypass_forms_pass_unmarked_through_the_runner(tmp_path):
    """An agent's call in a recorded bypass form gets no marker file, no
    permissionDecision and no updatedInput: the call passes as typed. Every
    LEDGER_BYPASS_FORMS entry goes through the runner as an agent
    (step3-hook-runner-fix-4-C2-2), together with the table's own runner
    cases — the single-quote junk of ORCH-2, the assignment programs of
    step3-hook-runner-fix-4-C1-1 and the %-/-/+ program starts of
    step3-hook-runner-fix-4-C2-1. The push form is no longer one of them:
    merge-guard now rightly blocks it (S4R-rules-12), tested beside."""
    project = helpers.make_project(tmp_path)
    ledger = [form for forms in LEDGER_BYPASS_FORMS.values() for form in forms]
    pushed = "10>/dev/null git push --force origin main"
    forms = [form for form in ledger if form != pushed] + [
        # ORCH-2: junk carrying a single quote between an outer pair of quotes
        "vsdd survey 'x'; id; 'y'",
        "vsdd survey ''$(id)''",
        'vsdd survey "x"',
        # step3-hook-runner-fix-4-C1-1: the program word is an assignment
        "FOO=/vsdd id",
        "a=b/vsdd survey",
        # step3-hook-runner-fix-4-C2-1: the program path starts with %, - or +
        "%/vsdd survey",
        "%1/vsdd x",
        "-x/vsdd survey",
        "+/vsdd survey",
    ]
    for command in dict.fromkeys(forms):  # the ledger holds some twice over
        payload = bash_call(project, command)
        payload["agent_id"] = "agent-7"
        payload["agent_type"] = "vsdd-factory:implementer"
        code, out, err = helpers.run_hook(payload, "gate", env=base_env(project))
        assert code == 0, (command, err)
        assert out == "", (command, out)  # no decision, no updatedInput
    assert helpers.root_markers(project) == []  # and no marker file


def test_fd_hugged_push_is_blocked_by_merge_guard(tmp_path):
    """The caller check leaves the fd-hugged push unmarked and unapproved,
    and merge-guard now stands between it and the exit (S4R-rules-12): the
    same text blocks with the push code."""
    project = helpers.make_project(tmp_path)
    command = "10>/dev/null git push --force origin main"
    payload = bash_call(project, command)
    payload["agent_id"] = "agent-7"
    payload["agent_type"] = "vsdd-factory:implementer"
    code, out, err = helpers.run_hook(payload, "gate", env=base_env(project))
    assert code == 10, (command, err)
    assert "push_to_guarded" in err, (command, err)
    assert helpers.root_markers(project) == []  # unmarked still


# --- an independent statement of the grammar (step3-hook-runner-fix-5-C2-2) --
#
# A second, hand-written statement of the plain-call grammar of
# rewrite/spec/topics/caller-identity.md, walked character by character. It
# shares no pattern, constant or helper with caller.py: the differential test
# below uses it as the oracle the allow-list's regular expression must agree
# with, so any widening or narrowing of a character class, a separator or a
# look-ahead there shows as a disagreement somewhere in the enumeration.

_CHECKER_BLANKS = " \t"
_CHECKER_LETTERS_DIGITS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
_CHECKER_SAFE = _CHECKER_LETTERS_DIGITS + "_@%+=:,./-"
# the twenty-one excluded characters of the spec, named one by one
_CHECKER_EXCLUDED = "\"$`\\;&|<>(){[]}*?~#!\n\r"


def _checker_blank(ch):
    return ch in _CHECKER_BLANKS


def _checker_quoted_end(text, i):
    """End of one single-quoted string starting at i, or None: it holds no
    single quote and no character below U+0020 or U+007F."""
    n = len(text)
    if i >= n or text[i] != "'":
        return None
    j = i + 1
    while j < n:
        ch = text[j]
        if ch == "'":
            return j + 1
        if ord(ch) < 0x20 or ord(ch) == 0x7F:
            return None
        j += 1
    return None  # never closed


def _checker_argument_ends(text, i):
    """End indices of every argument form starting at i: one single-quoted
    string alone; a bare-word prefix that is empty or ends with "=" followed
    by one single-quoted string; or a bare word of safe characters that does
    not start with "=" (nor with a tilde, which is no safe character)."""
    n = len(text)
    ends = []
    e = _checker_quoted_end(text, i)
    if e is not None:
        ends.append(e)
    if i < n and text[i] != "=":
        k = i
        while k < n and text[k] in _CHECKER_SAFE:
            if text[k] == "=":
                e = _checker_quoted_end(text, k + 1)
                if e is not None:
                    ends.append(e)
            k += 1
        k = i
        while k < n and text[k] in _CHECKER_SAFE:
            k += 1
            ends.append(k)
    return ends


def _checker_program_word(word):
    """`vsdd`, or a path of safe characters that ends in "/vsdd", holds no "="
    and begins with a letter, a digit, "_" "." or "/". Nothing asks for a
    character before the final "/vsdd": the bare "/vsdd" itself is such a
    path — safe characters, no "=", led by "/" — and an ordinary absolute
    path in sh, bash, dash and zsh (step3-hook-runner-fix-6-C1-1)."""
    if word == "vsdd":
        return True
    if not word.endswith("/vsdd"):
        return False
    head = word[:-5]  # possibly empty: the bare "/vsdd"
    if word[0] not in _CHECKER_LETTERS_DIGITS + "_./":
        return False
    return all(ch != "=" and ch in _CHECKER_SAFE for ch in head)


def _plain_vsdd_call(text):
    """Whether the whole text is one plain vsdd call."""
    n = len(text)
    i = 0
    while i < n and _checker_blank(text[i]):  # optional leading blanks
        i += 1
    j = i
    while j < n and not _checker_blank(text[j]):  # the program word
        j += 1
    if j == i or not _checker_program_word(text[i:j]):
        return False
    # zero or more arguments, each introduced by one or more blanks, then
    # optional trailing blanks. Dynamic programming from the right:
    # tail[p] says whether text[p:] finishes the argument list.
    tail = [False] * (n + 1)
    tail[n] = True
    for p in range(n - 1, j - 1, -1):
        if not _checker_blank(text[p]):
            continue
        a = p
        while a < n and _checker_blank(text[a]):
            a += 1
        if a == n:
            tail[p] = True  # trailing blanks
            continue
        for e in _checker_argument_ends(text, a):
            if tail[e]:
                tail[p] = True
                break
    return tail[j]


def _no_excluded_outside_quotes(text):
    """Whether text holds no excluded character outside a quoted string: the
    single quote toggles a quoted string, and the characters the spec
    excludes everywhere may sit inside one."""
    in_quote = False
    for ch in text:
        if ch == "'":
            in_quote = not in_quote
        elif not in_quote and ch in _CHECKER_EXCLUDED:
            return False
    return True


#: One base call per position of the grammar: the bare program, leading and
#: trailing blanks, a tab separator, every kind of program word, bare
#: arguments, a bare prefix ending in "=" before a quoted string, quoted
#: strings, safe characters, an empty quoted string.
DIFFERENTIAL_BASES = (
    "vsdd",
    "vsdd survey",
    "  vsdd\tsurvey  ",
    "./vsdd survey",
    "/usr/local/bin/vsdd survey",
    "/vsdd survey",
    "sub/vsdd survey",
    "1/vsdd survey",
    "_x/vsdd survey",
    "a%/vsdd survey",
    "vsdd survey --json --dry-run",
    "vsdd survey 'a b'",
    "vsdd survey 'aé'",
    "vsdd survey --name='two words'",
    "vsdd survey a+b=c:d,e.f/g-h_i%@x",
    "vsdd FOO='bar baz' survey",
    "vsdd survey 'x' y='z w'",
    "vsdd survey ''",
)

#: every ASCII character, then a non-ASCII letter, U+0085 and U+2028
DIFFERENTIAL_CHARACTERS = [chr(code) for code in range(0x80)] + ["\u00e9", "\u0085", "\u2028"]


def test_grammar_checker_agrees_with_the_allow_list():
    """step3-hook-runner-fix-5-C2-2: the regular expression and the
    independent checker are one grammar. For every base call, every index and
    every character of ASCII plus a few beyond, the character is inserted at
    the index or replaces the character at the index, and the character at the
    index is also deleted, and the two statements of the grammar must agree on
    the result — that pins every character class at every position, so no
    widening or narrowing of one can pass a test suite that asserts only
    hand-picked cases (step3-hook-runner-fix-6-C1-1: with "/vsdd survey" among
    the bases, the disagreement the checker's added length rule hid — a bare
    "/vsdd" — is pinned on the spec's side)."""
    for base in DIFFERENTIAL_BASES:
        assert _plain_vsdd_call(base), base
        assert caller._CALL_TEXT_RE.fullmatch(base), base
    for base in DIFFERENTIAL_BASES:
        for index in range(len(base) + 1):
            texts = [base[:index] + base[index + 1:]] if index < len(base) else []
            for ch in DIFFERENTIAL_CHARACTERS:
                texts.append(base[:index] + ch + base[index:])  # inserted
                texts.append(base[:index] + ch + base[index + 1:])  # replacing
            for text in texts:
                assert _plain_vsdd_call(text) == (
                    caller._CALL_TEXT_RE.fullmatch(text) is not None
                ), text


def test_program_word_refuses_every_excluded_character():
    """step3-hook-runner-fix-5-C2-2: the program word's own character classes
    are pinned against every excluded character and the single quote — a
    first or later character of the path may be none of them, so no widening
    of the first or the rest class can approve a program picked by a
    variable, a glob, a tilde or an equals expansion at run time."""
    for ch in _CHECKER_EXCLUDED + "'":
        for text in (f"./a{ch}b/vsdd survey", f"{ch}x/vsdd survey"):
            assert caller._CALL_TEXT_RE.fullmatch(text) is None, (ch, text)
            assert not caller.is_vsdd_call(_parse_call(text)), (ch, text)


def test_property_junk_from_excluded_characters_never_approves():
    """ORCH-2: strings built only from the excluded characters, dropped around
    a valid call, never sneak past the grammar — asserted as agreement: for
    every drawn text, over several fixed seeds, the regular expression and
    the independent checker must give the same answer whatever it is. A draw
    may legitimately be a plain call (a drawn single quote can close a
    span, and a tab is then a legal separator before a second quoted
    argument), so blanket refusal would assert a falsehood that passes only
    on the luck of its seeds (step3-hook-runner-fix-5-C2-1). An accepted draw
    is checked separately to hold no excluded character outside its quoted
    strings. The between-quotes draw inserts one character a quoted span
    refuses — a single quote, any control character below U+0020 or DEL — at
    a random index, so the span-interior rule is exercised from inside."""
    excluded = "\"$`\\;&|<>(){[]}*?~#!\n\r"
    # the alphabet the junk draws from holds the printable excluded characters
    # too, which sit legally inside a quoted span
    inside = excluded + "'" + "".join(chr(code) for code in range(0x20)) + "\x7f"
    # what a quoted span refuses: one of these rides in every between-quotes
    # draw, so the span-interior rule is exercised whatever the seed
    refused = "'" + "".join(chr(code) for code in range(0x20)) + "\x7f"
    base = "vsdd survey --flag value"
    assert _plain_vsdd_call(base)
    for seed in (20261124, 1, 2, 3, 42, 20261125, 4050, 19019):
        # the last two draw a correct plain call (a quote closes a span and a
        # tab separates the next quoted argument), so the accepted branch runs
        rng = random.Random(seed)
        for _ in range(200):
            junk = "".join(rng.choice(excluded) for _ in range(rng.randint(1, 8)))
            position = rng.randrange(4)
            if position == 0:
                text = junk + " " + base  # a junk word first
            elif position == 1:
                text = "vsdd" + junk + " survey --flag value"  # glued to the program
            elif position == 2:
                text = base + junk  # glued to the last word
            else:
                # between the quotes of one single-quoted argument, the span
                # held open by a plain x: one character the span refuses is
                # inserted at a random index and the draw's precondition
                # asserted
                inner = "x" + "".join(rng.choice(inside) for _ in range(rng.randint(1, 8)))
                at = rng.randrange(len(inner) + 1)
                inner = inner[:at] + rng.choice(refused) + inner[at:]
                text = base + " '" + inner + "'"
                assert any(
                    ch == "'" or ord(ch) < 0x20 or ord(ch) == 0x7F for ch in inner
                ), text
            accepted = caller._CALL_TEXT_RE.fullmatch(text) is not None
            assert accepted == _plain_vsdd_call(text), (text, accepted)
            if accepted:
                assert _no_excluded_outside_quotes(text), text


def test_every_control_character_is_refused_between_quotes():
    """ORCH-2, pinned per code point: a single-quoted string holds no single
    quote, no character below U+0020 and no U+007F. Six of these sit in
    REFUSED_CALLS; this pins all thirty-three, by the text grammar alone and
    through is_vsdd_call, so no widening of the quoted-span class can slip one
    through on the luck of a property draw."""
    for code in [*range(0x20), 0x7F]:
        text = f"vsdd survey 'a{chr(code)}b'"
        assert caller._CALL_TEXT_RE.fullmatch(text) is None, (code, text)
        assert not caller.is_vsdd_call(_parse_call(text)), (code, text)


def test_decisionless_rewrite_never_leaves(tmp_path):
    """PR4C-04 (HD7): the platform applies a rewrite whether or not a
    permission decision travels with it, so a decisionless rewrite is not
    advice — it silently changes what runs. The output step refuses to emit
    one: the rewriting rule's crash, raised loudly, no stdout JSON."""
    project = helpers.make_project(tmp_path)
    rewriting = rule_on(("SessionStart", ()), rewrites=True)
    with pytest.raises(DecisionlessRewrite) as raised:
        helpers.run_hook(
            helpers.payload("SessionStart/startup.json"), "notify",
            entries=(rewriting,), env=base_env(project),
        )
    assert "probe" in str(raised.value)  # the rewriting rule is named
    assert "decision" in str(raised.value)


def test_no_marker(tmp_path):
    project = helpers.make_project(tmp_path)

    # A compound command, a redirect, and a call outside any project: unmarked.
    for command in ("vsdd survey && echo done", "vsdd survey > out.txt"):
        run_gate(project, command)
    assert helpers.root_markers(project) == []

    # A blocked call: no marker either.
    env = base_env(project)
    payload = bash_call(project, "VSDD_CALLER=forged vsdd survey")
    code, out, err = helpers.run_hook(payload, "gate", env=env)
    assert code == 10
    assert helpers.root_markers(project) == []

    # A directory in no project: the call passes as typed, unmarked.
    outside = tmp_path / "plain"
    outside.mkdir()
    env = base_env(project)
    env["VSDD_FACTORY_ROOT"] = str(outside)
    payload = bash_call(outside, VSDD_CALL)
    code, out, err = helpers.run_hook(payload, "gate", env=env)
    assert code == 0 and out == ""  # unmarked, undecorated
    assert helpers.root_markers(outside) == []
    assert helpers.root_markers(project) == []


def test_typed_caller_blocked(tmp_path):
    project = helpers.make_project(tmp_path)
    env = base_env(project)
    payload = bash_call(project, "VSDD_CALLER=forged vsdd survey")
    code, out, err = helpers.run_hook(payload, "gate", env=env)
    assert code == 10
    assert "caller_forged" in err and "runner" in err


def test_typed_caller_blocked_through_launcher(tmp_path):
    from tests.hooks._helpers import INTERPRETERS

    project = helpers.make_project(tmp_path / "proj")
    stdin_text = json.dumps({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": "VSDD_CALLER=forged vsdd survey"},
        "session_id": "s1",
        "cwd": str(project),
    })
    launch_sh = helpers.PLUGIN / "py" / "launch.sh"
    for interpreter in INTERPRETERS.values():
        result = helpers.launch(
            interpreter, launch_sh, ["hook", "gate"], project,
            stdin_text=stdin_text,
            env={"VSDD_EVENTS_DIR": str(project / "events")},
        )
        assert result.returncode == 2, (interpreter, result.stderr)
        assert "caller_forged" in result.stderr


def test_write_failure(tmp_path):
    project = helpers.make_project(tmp_path)
    calls_dir = project / ".factory" / "local" / "hooks" / "calls"
    # An old marker is left for the reader: the failure must not remove it.
    calls_dir.mkdir(parents=True)
    old = calls_dir / "0ldmarketkey0ldmarketkey0ldmarketkey1234.json"
    old.write_text(json.dumps({"v": 1, "key": "0ldmarketkey0ldmarketkey0ldmarketkey1234"}))

    # Read-only calls directory: the marker cannot be written, the call blocks.
    calls_dir.chmod(stat.S_IRUSR | stat.S_IXUSR)
    try:
        code, out, err = run_gate(project)
        assert code == 10
        assert "caller_marker_unwritable" in err, err
        assert old.exists()  # an old marker is left for the reader
    finally:
        calls_dir.chmod(stat.S_IRWXU)


def test_marker_is_staged_outside_calls(tmp_path, monkeypatch):
    """The marker is written to hooks/tmp/ and renamed into calls/, so a kill
    before the rename never leaves a half file for the reader's prune
    (S3R-hook-runner-17)."""
    project = helpers.make_project(tmp_path)
    staged = {}
    real_replace = os.replace

    def spy(src, dst, **kwargs):
        staged["src"] = os.fspath(src)
        return real_replace(src, dst, **kwargs)

    monkeypatch.setattr(os, "replace", spy)
    code, out, err = run_gate(project)
    monkeypatch.undo()
    assert code == 0, err
    hooks_dir = project / ".factory" / "local" / "hooks"
    assert staged["src"].startswith(str(hooks_dir / "tmp" / "")), staged
    assert staged["src"].endswith(".json.tmp")
    assert helpers.root_markers(project)  # the rename still landed in calls/


def test_replace_failure_leaves_no_tmp_in_calls(tmp_path, monkeypatch):
    """A kill between the staged write and the rename (os.replace raising) is
    the marker unwritable, and calls/ holds no *.tmp file either way."""
    project = helpers.make_project(tmp_path)
    calls_dir = project / ".factory" / "local" / "hooks" / "calls"

    def killed(src, dst, **kwargs):
        raise OSError("killed before the rename")

    monkeypatch.setattr(os, "replace", killed)
    code, out, err = run_gate(project)
    monkeypatch.undo()
    assert code == 10
    assert "caller_marker_unwritable" in err, err
    assert not list(calls_dir.glob("*.tmp"))
    assert helpers.root_markers(project) == []


def test_structured_output_rewrite_is_allowed(tmp_path):
    """HD7: a rewrite on a StructuredOutput call is allow in any mode — the
    workflow step carries it, whatever permission_mode says. The main session
    (no agent id) is exercised: the recorded payload carries agent_id
    'wf-step-3', which would allow through the agent branch and so prove
    nothing about the StructuredOutput branch."""
    project = helpers.make_project(tmp_path)
    rewriting = rule_on(("PreToolUse", ("StructuredOutput",)), rewrites=True)
    payload = helpers.payload("PreToolUse/output-structured.json")
    payload["permission_mode"] = "default"
    payload["cwd"] = str(project)
    payload["agent_id"] = ""
    payload["agent_type"] = ""
    code, out, err = helpers.run_hook(payload, "gate", entries=(rewriting,),
                                      env=base_env(project))
    assert code == 0, err
    rendered = json.loads(out)["hookSpecificOutput"]
    assert rendered["permissionDecision"] == "allow"
    assert rendered["updatedInput"]["fixture"] is True


def test_permit_plus_vsdd_call_is_allowed(tmp_path):
    """HD7: a Permit standing beside the runner's rewrite makes the decision
    allow, for the main session too."""
    project = helpers.make_project(tmp_path)
    permitting = rule_on(("PreToolUse", ("Bash",)), kind="advisor",
                         module="tests.hooks.fixture_rules.permitting")
    payload = bash_call(project, VSDD_CALL)  # the main session, default mode
    code, out, err = helpers.run_hook(payload, "gate", entries=(permitting,),
                                      env=base_env(project))
    assert code == 0, err
    rendered = json.loads(out)["hookSpecificOutput"]
    assert rendered["permissionDecision"] == "allow"
    assert "VSDD_CALLER=" in rendered["updatedInput"]["command"]  # rewritten


def test_root_at_home_with_a_blocking_rule_is_silent(tmp_path):
    """The runner is inert at a root of home or /: no block is printed even
    when a blocking rule would select (SL1-14)."""
    rule = RuleEntry(
        id="probe", on=(("PreToolUse", ("Bash",)),), kind="gate", fail="closed",
        module="tests.hooks.fixture_rules.blocking",
    )
    for root in (os.environ["HOME"], "/"):
        env = base_env(tmp_path, {"HOME": os.environ["HOME"]})
        env["VSDD_FACTORY_ROOT"] = root
        payload = bash_call(tmp_path, "echo hi")
        payload["cwd"] = str(tmp_path)
        code, out, err = helpers.run_hook(payload, "gate", entries=(rule,), env=env)
        assert (code, out, err) == (0, "", ""), (root, code, out, err)


def test_marker_written_in_the_minus_c_project(tmp_path):
    """The marker root is the project the -C directory names, not the payload
    cwd: a second project's marker lands in the second project."""
    first = helpers.make_project(tmp_path)
    second_tmp = tmp_path / "elsewhere"
    second_tmp.mkdir()
    second = helpers.make_project(second_tmp)
    payload = bash_call(first, f"vsdd -C {second} survey")
    code, out, err = helpers.run_hook(payload, "gate", env=base_env(first))
    assert code == 0, err
    assert helpers.root_markers(second) != []  # marked there
    assert helpers.root_markers(first) == []  # and nowhere else


def test_leading_assignment_is_not_a_vsdd_call(tmp_path):
    """HD8: a leading assignment means the command is more than a vsdd call;
    it passes unmarked — no marker, no decision riding a rewrite."""
    project = helpers.make_project(tmp_path)
    payload = bash_call(project, "FOO=1 vsdd survey")
    code, out, err = helpers.run_hook(payload, "gate", env=base_env(project))
    assert code == 0 and out == "", (out, err)
    assert helpers.root_markers(project) == []


def test_quarantine_boundary_two_faults_skip_a_working_rule(tmp_path):
    """HD5: the quarantine threshold is two faults, and the quarantined rule
    is skipped entirely — here a rule that would block if it ran."""
    from tests.hooks.test_runner import base_env as runner_env, write_events

    project = helpers.make_project(tmp_path)
    rule = RuleEntry(
        id="probe", on=(("PreToolUse", ("Bash",)),), kind="gate", fail="closed",
        module="tests.hooks.fixture_rules.blocking",
    )
    env = runner_env(project, {"VSDD_FACTORY_HOOKS_QUARANTINE": "probe"})
    write_events(project, 2, "probe")
    code, out, err = helpers.run_hook(bash_call(project, "echo"), "gate",
                                      entries=(rule,), env=env)
    assert (code, out, err) == (0, "", ""), (code, out, err)  # skipped at two faults


def test_block_event_carries_the_story(tmp_path):
    """The rule.block event names the story whose worktree the call works in
    (SL1-50): the event is the survey's evidence."""
    project = helpers.make_project(tmp_path, story="S-1.02")
    rule = RuleEntry(
        id="probe", on=(("PreToolUse", ("Edit",)),), kind="gate", fail="closed",
        module="tests.hooks.fixture_rules.blocking",
    )
    payload = helpers.payload("PreToolUse/edit-main.json")
    payload["cwd"] = str(project)
    payload["tool_input"] = {
        "file_path": str(project / ".worktrees" / "S-1.02" / "src" / "app.py")
    }
    code, out, err = helpers.run_hook(payload, "gate", entries=(rule,), env=base_env(project))
    assert code == 10, err
    events = read_events(project)
    blocks = [event for event in events if event["type"] == "rule.block"]
    assert blocks and blocks[0]["story"] == "S-1.02"
