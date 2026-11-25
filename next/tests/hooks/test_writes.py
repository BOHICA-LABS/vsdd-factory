"""The shell-write reader (plan rules.md 3.5, test_writes.py): one table over
the programs, redirects, -t, a directory destination, globs, and the words
that are left out."""

from __future__ import annotations

import os

from vsdd.hooks.shell import parse
from vsdd.hooks.writes import write_targets


def _project(tmp_path) -> str:
    project = tmp_path / "proj"
    (project / "dir.d").mkdir(parents=True, exist_ok=True)
    (project / "src").mkdir(parents=True, exist_ok=True)
    (project / "src" / "one.o").write_text("", encoding="utf-8")
    (project / "src" / "two.o").write_text("", encoding="utf-8")
    return str(project)


def run(tmp_path, command: str, where: str | None = None) -> list[str]:
    """The write targets of one command's first segment."""
    parsed = parse(command, where if where is not None else _project(tmp_path))
    return list(write_targets(parsed.segments[0]))


def test_write_targets_table(tmp_path):
    base = _project(tmp_path)
    join = os.path.join
    cases = [
        # redirects
        ("echo hi > out.txt", [join(base, "out.txt")]),
        ("echo hi >> out.txt", [join(base, "out.txt")]),
        ("echo hi >| out.txt", [join(base, "out.txt")]),
        ("cmd 2> err.txt 1> out.txt", [join(base, "err.txt"), join(base, "out.txt")]),
        ("cmd &> both.txt", [join(base, "both.txt")]),
        # non-writing redirects
        ("wc < in.txt", []),
        ("cat <<EOF\nbody\nEOF", []),
        ("cmd 2>&1", []),
        ("cmd > /dev/null", []),
        # tee, rm, touch, truncate
        ("tee a.txt b.txt", [join(base, "a.txt"), join(base, "b.txt")]),
        ("rm -f old.txt other.txt", [join(base, "old.txt"), join(base, "other.txt")]),
        ("touch new.txt", [join(base, "new.txt")]),
        ("touch -r ref.bin new.bin", [join(base, "new.bin")]),        # S4R-rules-23
        ("touch --reference=ref.bin new.bin", [join(base, "new.bin")]),
        ("touch -d yesterday new.bin", [join(base, "new.bin")]),
        ("touch -t 202601010000 new.bin", [join(base, "new.bin")]),   # S4R-rules-23
        ("truncate -s 0 big.txt", [join(base, "big.txt")]),
        # sed and perl in place
        ("sed -i 's/a/b/' src/main.rs", [join(base, "src", "main.rs")]),
        ("sed -i.bak 's/a/b/' src/main.rs", [join(base, "src", "main.rs")]),
        ("sed --in-place 's/a/b/' src/main.rs", [join(base, "src", "main.rs")]),
        ("perl -pi -e 's/a/b/' src/main.rs", [join(base, "src", "main.rs")]),
        # perl's -E is -e's spelling too, bundled and separate (S4R-rules-23)
        ("perl -pi -E 's/a/b/' src/main.rs", [join(base, "src", "main.rs")]),
        ("perl -i -pE 's/a/b/' src/main.rs", [join(base, "src", "main.rs")]),
        # without -e/-E the first operand is the program file (S4R-rules-23)
        ("perl -i script.pl src/a.py", [join(base, "src", "a.py")]),
        ("perl -pi script.pl src/a.py", [join(base, "src", "a.py")]),
        # perl's -f takes no value and scripts nothing (S4R-rules-C2-3)
        ("perl -f -pi script.pl src/a.py", [join(base, "src", "a.py")]),
        ("perl -pi -f -e s/a/b/ src/a.py", [join(base, "src", "a.py")]),
        # sed with its script as an option: the script pairs are no operands
        # (S4R-rules-1)
        ("sed -i -e 's/a/b/' src/a.py", [join(base, "src", "a.py")]),
        ("sed --in-place -e 's/a/b/' src/a.py", [join(base, "src", "a.py")]),
        ("sed -i.bak -e 's/a/b/' src/a.py", [join(base, "src", "a.py")]),
        ("sed -i -f script.sed src/a.py src/b.py",
         [join(base, "src", "a.py"), join(base, "src", "b.py")]),
        ("sed -i -e 's/a/b/' -e 's/c/d/' src/a.py src/b.py",
         [join(base, "src", "a.py"), join(base, "src", "b.py")]),
        ("sed --in-place --expression='s/a/b/' src/a.py", [join(base, "src", "a.py")]),
        # perl whose bundled letters are not in place (S4R-rules-23)
        ("perl -MData::Dumper -e 'print' src/lib.pl", []),
        ("perl -MFile::Find::Inline -ne 'print' src/lib.pl", []),
        ("perl -i.bak -pe 's/a/b/' src/main.rs", [join(base, "src", "main.rs")]),
        ("perl -MInline -pi -e 's/a/b/' src/main.rs", [join(base, "src", "main.rs")]),
        # cp, mv, ln
        ("cp a.txt b.txt", [join(base, "b.txt")]),
        ("cp a.txt b.txt dir.d", [join(base, "dir.d", "a.txt"), join(base, "dir.d", "b.txt")]),
        ("cp a.txt -t dir.d", [join(base, "dir.d", "a.txt")]),
        ("cp a.txt --target-dir=dir.d", [join(base, "dir.d", "a.txt")]),
        ("cp a.txt --target-directory=dir.d", [join(base, "dir.d", "a.txt")]),  # S4R-rules-23
        ("cp a.txt --target-directory dir.d", [join(base, "dir.d", "a.txt")]),
        ("mv old.txt new.txt", [join(base, "old.txt"), join(base, "new.txt")]),
        ("ln -sf a.txt link.txt", [join(base, "link.txt")]),
        # dd
        ("dd if=in.img of=out.img", [join(base, "out.img")]),
        # the program's own name, wherever it is mounted (S4R-rules-7), and
        # the wrapper's own options are skipped, not read as the program
        ("env -i sed -i 's/a/b/' src/main.rs", [join(base, "src", "main.rs")]),
        ("nice -n 5 sed -i 's/a/b/' src/main.rs", [join(base, "src", "main.rs")]),
        ("/usr/bin/env sed -i 's/a/b/' src/main.rs", [join(base, "src", "main.rs")]),
        ("/usr/bin/sed -i 's/a/b/' src/main.rs", [join(base, "src", "main.rs")]),
        ("/usr/bin/touch new.txt", [join(base, "new.txt")]),
    ]
    for command, want in cases:
        got = run(tmp_path, command)
        assert got == want, (command, got)


def test_destination_directory_is_optional(tmp_path):
    base = _project(tmp_path)
    # A destination that exists as a directory takes each source's base name;
    # one that does not exist is the file written.
    got = run(tmp_path, "cp a.txt fresh.txt")
    assert got == [os.path.join(base, "fresh.txt")]


def test_globs_expand(tmp_path):
    base = _project(tmp_path)
    got = run(tmp_path, "rm build/*.o src/*.o")
    assert sorted(os.path.basename(path) for path in got) == ["one.o", "two.o"]
    # A glob with no hits contributes nothing and never raises.
    assert run(tmp_path, "rm nothing/*.missing") == []


def test_expansions_and_unknown_where(tmp_path):
    base = _project(tmp_path)
    # A word holding an expansion is left out.
    assert run(tmp_path, "echo $msg > $out") == []
    assert run(tmp_path, "echo hi > `mktemp`") == []
    # A relative word with an unknown where is left out; an absolute one is not.
    assert run(tmp_path, "echo hi > out.txt", where="") == []
    assert run(tmp_path, "echo hi > /tmp/log", where="") == ["/tmp/log"]


def test_never_raises():
    """Whatever the segment holds, write_targets answers."""
    parsed = parse("cd somewhere && ((unclosed", "/proj")
    for segment in parsed.segments:
        write_targets(segment)
