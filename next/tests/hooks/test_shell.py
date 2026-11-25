"""The shell splitter: joiners, quoting, redirects, cd, -C, opaque constructs
(plan 7, the parse table)."""

from __future__ import annotations

from vsdd.hooks.shell import Redirect, parse


def test_joiners_and_order():
    command = parse("a1 && a2 || a3; a4 | a5 & a6\na7", "/base")
    joiners = [segment.joiner for segment in command.segments]
    assert joiners == ["", "&&", "||", ";", "|", "&", "\n"]
    programs = [segment.program for segment in command.segments]
    assert programs == ["a1", "a2", "a3", "a4", "a5", "a6", "a7"]


def test_quoting():
    command = parse("echo 'a b' \"c d\" plain", "/base")
    segment = command.segments[0]
    assert segment.args == ("a b", "c d", "plain")

    quoted = parse("git commit -m 'fix: the thing' -m \"body\"", "/base")
    assert quoted.segments[0].args == ("commit", "-m", "fix: the thing", "-m", "body")


def test_redirects():
    command = parse("grep -r pattern > out.txt 2> err.txt", "/base")
    segment = command.segments[0]
    assert segment.redirects[0].op == ">"
    assert segment.redirects[0].target == "out.txt"
    assert segment.redirects[1] == Redirect("2>", "err.txt")
    assert segment.program == "grep"

    fd = parse("git 2>&1 log", "/base")
    assert fd.segments[0].redirects[0] == Redirect("2>&", "1")

    append = parse("echo x >> log", "/base")
    assert append.segments[0].redirects == (Redirect(">>", "log"),)


def test_cd_sets_where_for_later_segments():
    command = parse("cd /tmp && rm -rf x && ls /", "/base")
    where = [segment.where for segment in command.segments]
    assert where == ["/base", "/tmp", "/tmp"]

    relative = parse("cd sub && ls", "/base")
    assert relative.segments[1].where == "/base/sub"

    unknowable = parse("cd $WORK && ls", "/base")
    assert unknowable.segments[1].where == ""  # not guessed

    cd_itself_keeps_previous = parse("cd /tmp && ls", "/base")
    assert cd_itself_keeps_previous.segments[0].where == "/base"


def test_minus_c_names_their_own_segment():
    command = parse("git -C repo status && git log", "/base")
    assert command.segments[0].where == "/base/repo"
    assert command.segments[1].where == "/base"

    fused = parse("git -Crepo status", "/base")
    assert fused.segments[0].where == "/base/repo"

    tool = parse("vsdd -C repo2 survey", "/base")
    assert tool.segments[0].where == "/base/repo2"

    chained = parse("cd /tmp && vsdd -C r survey", "/base")
    assert chained.segments[1].where == "/tmp/r"


def test_opaque_constructs():
    for text in (
        "eval dangerous",
        "sh -c 'echo hi'",
        "xargs -n1 echo",
        "echo $(rm -rf /)",
        "echo `date`",
        "cat <<EOF\nbody; rm x\nEOF\necho done",
    ):
        command = parse(text, "/base")
        assert command.opaque, text

    # A subshell the parser does not enter: its cd leaks nowhere.
    command = parse("(cd /etc && rm x); echo after", "/base")
    assert command.opaque
    assert command.segments[-1].program == "echo"
    assert command.segments[-1].where == "/base"


def test_assignments():
    command = parse("FOO=1 BAR='two words' vsdd survey", "/base")
    segment = command.segments[0]
    assert segment.program == "vsdd"
    assert segment.assignments == (("FOO", "1"), ("BAR", "two words"))
    assert segment.args == ("survey",)


def test_comments_and_continuations():
    command = parse("echo a # inline comment\n# whole line\necho b", "/base")
    programs = [segment.program for segment in command.segments]
    assert programs == ["echo", "echo"]
    assert command.segments[0].args == ("a",)

    continued = parse("echo \\\nhello", "/base")
    assert continued.segments[0].args == ("hello",)


def test_parse_never_raises():
    broken = parse("echo 'unterminated", "/base")
    assert broken.segments  # degraded, not crashed
    empty = parse("", "/base")
    assert empty.segments == ()


def test_substitution_in_any_word_is_opaque():
    """A substitution in the program word, an assignment value or a redirect
    target runs anything: the segment is opaque, not judged word by word."""
    for text in (
        "$(echo rm) -rf /",        # in the program word
        "X=$(rm -rf /) ls",        # in an assignment value
        "echo > $(rm x)",          # in a redirect target
        "echo `date`",             # a backtick span, as before
    ):
        command = parse(text, "/base")
        assert command.opaque, text
        assert command.segments[0].opaque, text

    # Plain expansions and globs still are not opaque; they only make a
    # `where` unknowable.
    plain = parse("echo $HOME *.txt", "/base")
    assert not plain.opaque
    quoted = parse('echo "$(safe)"', "/base")
    assert quoted.opaque


def test_arithmetic_is_not_opaque_unless_it_runs_something():
    """$((...)) evaluates; it runs nothing, so it is no substitution and the
    segment stays judgeable (S3R-hook-runner-C1-1: a vsdd call with an
    arithmetic argument keeps its caller marker). A nested command
    substitution or backtick runs code: opaque."""
    plain = parse("echo $((1+2))", "/base")
    assert not plain.opaque
    assert not plain.segments[0].opaque
    assert plain.segments[0].args == ("$((1+2))",)

    in_quotes = parse('echo "n=$((1+2))"', "/base")
    assert not in_quotes.opaque

    nested = parse("echo $(($(wc -l)+1))", "/base")
    assert nested.opaque

    backticked = parse("echo $((1+`echo 2`))", "/base")
    assert backticked.opaque

    # An arithmetic argument does not cost a vsdd call its marker:
    argument = parse("vsdd survey --depth $((1+2))", "/base")
    assert not argument.opaque


def test_substitution_beside_arithmetic_in_one_word_is_opaque():
    """S3R-hook-runner-C2-1: a command substitution or a backtick sharing its
    word with an arithmetic span keeps the word a substitution — the
    arithmetic span must not unsee what came before it. And ``$((cmd) )`` is
    no arithmetic at all: sh and bash run it as a command substitution."""
    for text in (
        "echo $(id)$((1))",
        'echo "$(id)$((1))"',
        "echo `id`$((1))",
        "X=$(id)$((1)) ls",         # in an assignment value
        "echo > $(id)$((1))",       # in a redirect target
        "vsdd survey $((id) )",     # $((cmd) ): a command substitution
        "echo $((echo PWNED)|(cat))",   # sh and bash end the span at the )
        "echo $((a)&&(b))",             # that closes its second ( and reparse
        "vsdd survey $((id)|(id))",     # the rest: it runs code
        'vsdd survey "$((id)|(id))"',
        "X=${y:-$(id)} ls",         # a ${...} span holding a substitution
        "echo > ${y:-$(id)}",
        "vsdd survey ${y:-'}'$(id)}",   # a quote hides the span's real end
        "vsdd survey ${(e):-\\$\\(id\\)}",  # a zsh nest the loop does not read
    ):
        command = parse(text, "/base")
        assert command.opaque, text
        assert command.segments[0].opaque, text

    # Plain arithmetic — also beside other text in the word, and grouped —
    # stays judgeable:
    for text in ("echo $((1+2))", 'echo "n=$((1+2))"', "echo a$((1+2))b",
                 "echo $(((1+2)))", "echo $(( (1+2)*3 ))", "X=${y:-$((1+2))} ls"):
        command = parse(text, "/base")
        assert not command.opaque, text


def test_parameter_span_inside_double_quotes_counts_substitutions():
    """S3R-hook-runner-C2-1: the plain-text rule for a ${...} span holds inside
    double quotes as well. A quote nested in the span hides its real end from
    the first '}' while the shell reads past it, a setter planting code behind
    an escaped \\$ feeds a plain-looking reader, and the zsh nest the loop does
    not read must not pass for plain either — every form runs a hidden
    command, so each parses opaque."""
    for text in (
        'vsdd survey "${x:-"\'"}"$(id)\'\'',   # sh, bash, dash, zsh run id
        'vsdd survey "${x:-\'"\'}"$(id)\'\'',  # sh and bash run id
        'vsdd survey "${(e):-\\$(id)}"',       # a zsh nest the loop does not read
        'vsdd survey "${x:=\\$(id)}"',         # a setter plants code
        'vsdd survey "${x:=\\$(id)}${x@P}"',   # ... a plain-looking reader runs
    ):
        command = parse(text, "/base")
        assert command.opaque, text
        assert command.segments[0].opaque, text

    # A span that is plainly a name, an operator and nothing executable stays
    # judgeable inside quotes:
    plain = parse('vsdd survey "${x:-a b}"', "/base")
    assert not plain.opaque
    assert plain.segments[0].args[-1] == "${x:-a b}"


def test_dollar_quote_counts_a_substitution():
    """step3-hook-runner-fix-3-C2-1: bash and zsh read $'...' as ANSI-C
    quoting, where \\' does not end the string, while dash reads the POSIX way.
    The shells disagree on where the string ends, so a $' outside double quotes
    is never plain text: both forms parse opaque, not as one vsdd call."""
    for text in (
        "vsdd survey $'\\'' ; id #'",
        "vsdd survey $'\\'';id;'x'",
    ):
        command = parse(text, "/base")
        assert command.opaque, text
        assert command.segments[0].opaque, text


def test_prompt_and_indirect_parameter_operators_count_substitutions():
    """step3-hook-runner-fix-3-C2-2: '@' and '!' are no plain parameter
    characters. bash 4.4+'s ${name@P} expands the value as a prompt string,
    which runs any $(...) in it (${BASH_EXECUTION_STRING@P} holds the command
    itself), and ${!x} is indirect expansion through array subscripts."""
    for text in (
        "vsdd survey '$(id)' ${BASH_EXECUTION_STRING@P}",
        "vsdd survey ${x@P}",
        "vsdd survey ${!x}",
        'vsdd survey "${x:=a[\\$(id)]}" ${!x}',
        'vsdd survey "${x:=a[\\$(id)]}" $((x))',
    ):
        command = parse(text, "/base")
        assert command.opaque, text
        assert command.segments[0].opaque, text


def test_leading_redirect_before_the_program():
    """S3B-hook-runner-3: POSIX lets the redirects precede the words, so a
    leading redirect lands in the redirect list and the program is the first
    real word — never an fd digit or an operator."""
    leading = parse("2>log cmd", "/base")
    segment = leading.segments[0]
    assert segment.program == "cmd"
    assert segment.redirects == (Redirect("2>", "log"),)

    bare = parse(">out cmd arg", "/base")
    segment = bare.segments[0]
    assert segment.program == "cmd"
    assert segment.args == ("arg",)
    assert segment.redirects == (Redirect(">", "out"),)

    # The common form still parses the same way:
    trailing = parse("cmd 2>log", "/base")
    assert trailing.segments[0].redirects == (Redirect("2>", "log"),)

    # An fd number is any run of digits (step3-hook-runner-fix-3-C1-3):
    wide = parse("10>/dev/null cmd arg", "/base")
    segment = wide.segments[0]
    assert segment.program == "cmd"
    assert segment.args == ("arg",)
    assert segment.redirects == (Redirect("10>", "/dev/null"),)


def test_brackets_split_or_are_opaque():
    """step3-hook-runner-fix-3-C1-1: a '[' is no construct of its own, so the
    word loop reads past it — a metacharacter inside brackets splits the
    words as every shell sees them, and a substitution or backtick inside
    brackets shows. No bracket form hides a second command in one judgeable
    word."""
    for text in (
        "vsdd survey [ ; id ; ]",
        "vsdd survey [;id;]",
        "vsdd survey [ && id ]",
        "vsdd survey a[b|id]",
        "vsdd survey [$(id)]",
        "vsdd survey [`id`]",
        "vsdd survey $[$(id)]",
        "vsdd survey $['$(id)']",  # bash runs it; the quote must not hide it
        'vsdd survey "$[a[\\$(id)]]"',  # $[ inside quotes too; \$ hides nothing
    ):
        command = parse(text, "/base")
        assert command.opaque or len(command.segments) > 1, text

    # A glob bracket still judges:
    glob = parse("vsdd survey a[bc]", "/base")
    assert not glob.opaque
    assert len(glob.segments) == 1


def test_carriage_return_is_a_word_character():
    """step3-hook-runner-fix-3-C1-2: sh, bash, dash and zsh read a carriage
    return as an ordinary word character, so it neither ends a word nor
    starts a comment: '# ; more' after it runs as a second command."""
    command = parse("vsdd survey \r#; id", "/base")
    assert len(command.segments) == 2
    assert not command.opaque
    assert [segment.program for segment in command.segments] == ["vsdd", "id"]

    glued = parse("echo a\rb", "/base")
    assert glued.segments[0].args == ("a\rb",)


def test_where_unknown_forms():
    """A `where` is "" rather than a wrong fact: an expanded or shorthand
    target, or a bare cd, names no guessable directory."""
    assert parse("cd ~/proj && git status", "/base").segments[1].where == ""
    assert parse("git -C ~/x status", "/base").segments[0].where == ""
    assert parse("cd foo$BAR && ls", "/base").segments[1].where == ""
    assert parse("cd - && ls", "/base").segments[1].where == ""
    assert parse("cd && ls", "/base").segments[1].where == ""  # a bare cd goes home
    # Options to cd are skipped; `--` ends them. The operand decides.
    assert parse("cd -P /tmp && git status", "/base").segments[1].where == "/tmp"
    assert parse("cd -L sub && ls", "/base").segments[1].where == "/base/sub"
    assert parse("cd -e /tmp && ls", "/base").segments[1].where == "/tmp"
    assert parse("cd -- sub && ls", "/base").segments[1].where == "/base/sub"
    assert parse("cd -P -- /tmp && ls", "/base").segments[1].where == "/tmp"
    assert parse("cd -- -P && ls", "/base").segments[1].where == "/base/-P"  # an operand after --
    assert parse("cd -P && ls", "/base").segments[1].where == ""  # options only: home
    # The directory-stack builtins move where their stack decides: "".
    assert parse("pushd /tmp && git status", "/base").segments[1].where == ""
    assert parse("popd && git status", "/base").segments[1].where == ""
    # Literal relatives and absolutes resolve as before:
    assert parse("cd sub && ls", "/base").segments[1].where == "/base/sub"
    assert parse("cd /tmp && ls", "/base").segments[1].where == "/tmp"


def test_for_and_case_headers_are_opaque():
    """A for header is no command: the loop variable is not a program, so the
    segment is seen whole. A case header dissolves at its `)`, and the parser
    judges the bodies, which are real commands."""
    command = parse("for f in a b; do rm -rf $f; done", "/base")
    assert command.segments[0].opaque  # the header
    assert not command.segments[1].opaque  # the body is a real command
    assert command.segments[1].program == "rm"

    case = parse("case $1 in a) go ;; esac", "/base")
    assert [s.program for s in case.segments] == ["go", ""]


def test_ampersand_redirect_ops():
    """&> and &>> are redirects of the segment, not a background joiner
    followed by another segment's redirection."""
    both = parse("echo x &> all.txt", "/base")
    segment = both.segments[0]
    assert segment.program == "echo"
    assert segment.redirects == ((Redirect("&>", "all.txt"),))
    assert [s.program for s in both.segments] == ["echo"]

    append = parse("echo x &>> all.txt", "/base")
    assert append.segments[0].redirects == (Redirect("&>>", "all.txt"),)
    assert len(append.segments) == 1

    # A bare & still joins:
    joiner = parse("echo a & echo b", "/base")
    assert [s.program for s in joiner.segments] == ["echo", "echo"]
    assert joiner.segments[1].joiner == "&"


def test_wrapper_leading_operand_parses_to_the_wrapped_program():
    """S4R-rules-C2-2: the leading operand of a wrapper — timeout's duration
    — is consumed by its own table, and the wrapped program parses with its
    arguments; a tuple read in the operand used to crash the parse into one
    opaque segment."""
    for text in ("timeout 10 git commit -m x", "timeout 5 true && git status"):
        command = parse(text, "/base")
        assert not command.opaque, text
    timeout = parse("timeout 10 git commit -m x", "/base")
    assert timeout.segments[0].program == "git"
    assert timeout.segments[0].args == ("commit", "-m", "x")
    assert timeout.segments[0].where == "/base"

    chained = parse("cd /tmp && timeout 10 git commit -m x", "/base")
    assert chained.segments[1].program == "git"
    assert chained.segments[1].where == "/tmp"

    stdbuf = parse("stdbuf -o 0 git log", "/base")
    assert stdbuf.segments[0].program == "git"
    assert stdbuf.segments[0].args == ("log",)

    sudo = parse("sudo -u root git status", "/base")
    assert sudo.segments[0].program == "git"
    assert sudo.segments[0].args == ("status",)


def test_directory_changing_wrapper_options_move_the_where():
    """S4R-rules-C2-2: each wrapper owns its long-value prefixes, and the
    directory-changing ones move the wrapped program's where — env -C/
    --chdir and sudo -D/--chdir resolve it, sudo -R/--chroot leaves it
    unknowable. Corrected: sudo's -R is --chroot, its -D is --chdir."""
    assert parse("env --chdir=/tmp git commit -m x", "/base").segments[0].where == "/tmp"
    assert parse("env -C /tmp git commit -m x", "/base").segments[0].where == "/tmp"
    assert parse("sudo -D /tmp git status", "/base").segments[0].where == "/tmp"
    assert parse("sudo --chdir=/tmp git status", "/base").segments[0].where == "/tmp"
    assert parse("sudo -R /srv git status", "/base").segments[0].where == ""
    # A wrapper option unknown to its own table is opaque, as before.
    assert parse("env --bogus git status", "/base").segments[0].opaque


def test_absolute_cd_to_a_missing_directory_is_unknown():
    """step4-rules-fix-3-C1-5: an absolute cd operand that is not a
    directory at parse time names no place — the worktree an earlier
    segment is about to create included. A relative one lands where it
    resolves or nowhere."""
    assert parse("cd /definitely/not/here && git status", "/base").segments[1].where == ""
    assert parse("cd sub && ls", "/base").segments[1].where == "/base/sub"


def test_a_subshell_holding_a_cd_is_unplaced():
    """S4B-rules-2: the subshell's cd runs inside the subshell, so its
    segment's where is "" rather than the outer directory."""
    command = parse("(cd /etc && git commit -m x)", "/base")
    assert command.segments[0].where == ""
    assert command.segments[0].opaque
    # A subshell with no cd keeps the outer directory.
    calm = parse("(make && git status)", "/base")
    assert calm.segments[0].where == "/base"


def test_a_function_body_is_judged_at_its_call_site():
    """step4-rules-fix-3-C1-5: a git call in a { ... } body is not plain, and
    its where is "" — the body runs wherever the function is called."""
    command = parse("f() { git commit -m x; }; cd /root && f", "/base")
    body = command.segments[2]
    assert body.opaque
    assert body.where == ""


def test_minus_c_options_thread_in_order():
    """step4-rules-fix-3-C1-5: every -C is applied in order, the last one
    standing."""
    command = parse("git -C . -C ../.. commit", "/base/.worktrees/S")
    assert command.segments[0].where == "/base"
    expanded = parse("git -C . -C $ELSEWHERE commit", "/base/.worktrees/S")
    assert expanded.segments[0].where == ""


def test_leading_reserved_words_strip():
    """The program of a compound command's part is its real program, not the
    reserved word that introduces it."""
    command = parse("if cd /tmp; then git commit; fi", "/base")
    programs = [segment.program for segment in command.segments]
    assert programs == ["cd", "git", ""]
    assert command.segments[1].where == "/tmp"  # the cd is threaded through

    negated = parse("! git push", "/base")
    assert negated.segments[0].program == "git"
    assert negated.segments[0].args == ("push",)
