"""The tests-first rule in process (plan rules.md section 7).

The row is passed as ``entries`` until it lands in the registry; the runner
then loads the rule module by the row's id. The story worktree is a real
survey's, from ``rule_projects.story_project``.
"""

from __future__ import annotations

from tests.hooks import runner_helpers as helpers
from tests.hooks import rule_projects
from tests.hooks.runner_helpers import run_hook
from tests.hooks.test_runner import base_env
from vsdd.hooks.model import RuleEntry

ATOMS = ("Edit", "Write", "MultiEdit", "NotebookEdit", "Bash")

ROW = RuleEntry(
    id="tests-first",
    on=(("PreToolUse", ATOMS),),
    kind="gate",
    fail="closed",
    spike_exempt=True,
    summary="fixture row for the tests-first tests",
)


def call(p, agent_type: str, *, file_path: str | None = None, command: str | None = None,
         cwd=None):
    payload = {
        "hook_event_name": "PreToolUse",
        "tool_name": "Edit" if file_path else "Bash",
        "session_id": "sess-rules",
        "agent_type": agent_type,
        "cwd": str(cwd if cwd is not None else p.worktree),
        "tool_input": (
            {"command": command}
            if command
            else {"file_path": file_path, "old_string": "a", "new_string": "b"}
        ),
    }
    code, out, err = run_hook(payload, "gate", entries=(ROW,),
                              env=base_env(p.root))
    return code, err


def test_source_before_and_after_red(tmp_path):
    p = rule_projects.story_project(tmp_path)
    source = str(p.worktree / "src" / "lib.rs")

    # Before red: the implementer and the main session are blocked, the
    # test writer's before-red grant is not.
    assert call(p, "vsdd-factory:implementer", file_path=source)[0] == 10
    assert call(p, "", file_path=source)[0] == 10
    code, err = call(p, "vsdd-factory:test-writer", file_path=source)
    assert code == 0, err

    rule_projects.red(p)
    # After red: the implementer writes source, the test writer's grant is spent.
    code, err = call(p, "vsdd-factory:implementer", file_path=source)
    assert code == 0, err
    code, err = call(p, "vsdd-factory:test-writer", file_path=source)
    assert code == 10, err
    assert "red_already_recorded" in err, err
    code, err = call(p, "", file_path=source)
    assert code == 0, err

    # The record holds across the implementer's own commits: the lookup is
    # not tied to the head commit, so a real commit happens here
    # (S4R-rules-18).
    rule_projects.commit(p)
    later = str(p.worktree / "src" / "other.rs")
    assert call(p, "vsdd-factory:implementer", file_path=later)[0] == 0


def test_frozen_tests(tmp_path):
    p = rule_projects.story_project(tmp_path)
    rule_projects.freeze(p.root, ["tests/test_sample.py"])
    rule_projects.red(p, frozen=["tests/test_sample.py"])

    frozen = str(p.worktree / "tests" / "test_sample.py")
    free = str(p.worktree / "tests" / "test_other.py")

    # The test writer may change a frozen test (its row grants tests:frozen).
    code, err = call(p, "vsdd-factory:test-writer", file_path=frozen)
    assert code == 0, err
    # The implementer and the verifier get red_test_frozen on it...
    for agent in ("vsdd-factory:implementer", "vsdd-factory:verifier", ""):
        code, err = call(p, agent, file_path=frozen)
        assert code == 10, (agent, err)
        assert "red_test_frozen" in err, err
    # ...and an unfrozen test file passes.
    code, err = call(p, "vsdd-factory:verifier", file_path=free)
    assert code == 0, err
    # src/parser/tests.rs is a test file: the tests glob wins the precedence.
    nested = str(p.worktree / "src" / "parser" / "tests.rs")
    code, err = call(p, "vsdd-factory:test-writer", file_path=nested)
    assert code == 0, err
    # The fix sends the implementer and the verifier back blocked, with the
    # test named for data.test_defects (S4R-rules-16) — not to the refreeze
    # command their rows cannot run.
    code, err = call(p, "vsdd-factory:implementer", file_path=frozen)
    assert "data.test_defects" in err, err
    assert "tests/test_sample.py" in err, err
    assert "Instead: Return blocked again" in err, err


def test_refrozen_set_stays_frozen(tmp_path):
    p = rule_projects.story_project(tmp_path)
    rule_projects.freeze(p.root, ["tests/test_sample.py"])
    rule_projects.red(p, frozen=["tests/test_sample.py"])

    # The test writer changes one frozen test; the re-freeze record lists the
    # whole set, one file added in the fix included (P3SEAM-15). A real
    # commit separates the two records, so both exist and the frozen set is
    # the latest record's (S4R-rules-17).
    rule_projects.commit(p)
    rule_projects.freeze(p.root, ["tests/test_added.py"])
    rule_projects.red(
        p,
        frozen=["tests/test_sample.py", "tests/test_added.py"],
        refreeze=True,
    )

    for name in ("tests/test_sample.py", "tests/test_added.py"):
        frozen = str(p.worktree / name)
        code, err = call(p, "vsdd-factory:implementer", file_path=frozen)
        assert code == 10, (name, err)
        assert "red_test_frozen" in err, err


def test_frozen_test_through_symlinked_parent(tmp_path):
    """A parent directory that is a symlink — macOS /tmp, a linked home —
    leaves the resolved path in another spelling of the same worktree; the
    frozen test is still the frozen test (S4R-rules-2). The call goes through
    Bash, whose write targets keep the spelling they were given: an ``Edit``
    target is resolved by the runner, and both spellings would coincide."""
    p = rule_projects.story_project(tmp_path)
    rule_projects.freeze(p.root, ["tests/test_sample.py"])
    rule_projects.red(p, frozen=["tests/test_sample.py"])

    alias = tmp_path / "aliased"
    alias.symlink_to(tmp_path, target_is_directory=True)
    worktree = alias / p.worktree.relative_to(tmp_path)
    assert worktree != p.worktree  # the two spellings really differ
    command = "echo x > tests/test_sample.py"
    code, err = call(
        p, "vsdd-factory:verifier", command=command, cwd=worktree
    )
    assert code == 10, err
    assert "red_test_frozen" in err, err


def test_story_lookups_taken_once(tmp_path, monkeypatch):
    """The per-story lookups (the kind's red gate, the red record) are taken
    once per story, not once per written path: one multi-path Bash write in
    one story costs one lookup of each (S4R-rules-19)."""
    p = rule_projects.story_project(tmp_path)
    rule_projects.red(p)

    from vsdd.state import gate_evidence, story_unit

    counts = {"kind": 0, "evidence": 0}
    real_kind = story_unit.story_kind
    real_evidence = gate_evidence.evidence

    def counting_kind(project, story):
        counts["kind"] += 1
        return real_kind(project, story)

    def counting_evidence(project, gate, story):
        counts["evidence"] += 1
        return real_evidence(project, gate, story)

    monkeypatch.setattr(story_unit, "story_kind", counting_kind)
    monkeypatch.setattr(gate_evidence, "evidence", counting_evidence)

    sources = " ".join(
        f"{p.worktree}/src/{name}.rs" for name in ("one", "two", "three")
    )
    command = f"sed -i 's/a/b/' {sources}"
    code, err = call(p, "vsdd-factory:implementer", command=command)
    assert code == 0, err  # after red, every path passes and is judged
    assert counts["kind"] == 1, counts
    assert counts["evidence"] == 1, counts


def test_bash_writes(tmp_path):
    p = rule_projects.story_project(tmp_path)
    before = f"sed -i 's/a/b/' {p.worktree}/src/main.rs"
    before_e = f"sed -i -e 's/a/b/' {p.worktree}/src/main.rs"
    redirect = f"echo x > {p.worktree}/tests/test_sample.py"
    rm_glob = f"rm {p.worktree}/build/*.o"

    # Before red, the implementer's sed on source is the edit it stands for,
    # with the script as an option too (S4R-rules-1).
    code, err = call(p, "vsdd-factory:implementer", command=before)
    assert code == 10, err
    assert "no_red_evidence" in err, err
    code, err = call(p, "vsdd-factory:implementer", command=before_e)
    assert code == 10, err
    assert "no_red_evidence" in err, err

    rule_projects.freeze(p.root, ["tests/test_sample.py"])
    rule_projects.red(p)

    # After red the same sed passes, a redirect into a frozen test does not,
    # and rm of an unclassified glob is free.
    code, err = call(p, "vsdd-factory:implementer", command=before)
    assert code == 0, err
    code, err = call(p, "vsdd-factory:implementer", command=before_e)
    assert code == 0, err
    code, err = call(p, "vsdd-factory:implementer", command=redirect)
    assert code == 10, err
    assert "red_test_frozen" in err, err
    code, err = call(p, "vsdd-factory:implementer", command=rm_glob)
    assert code == 0, err


def test_unreadable_write_is_the_edit_it_could_be(tmp_path):
    """A write the parse cannot read — a writer behind an opaque segment —
    is the edit it could be (S4R-rules-7): like a source write before red,
    like a frozen-test write after it; the same write read plainly passes
    after red as the source edit it is."""
    p = rule_projects.story_project(tmp_path)
    implementer = "vsdd-factory:implementer"
    opaque = 'sh -c "sed -i s/a/b/ src/lib.rs"'
    split = 'env -S "sed -i s/a/b/ src/lib.rs"'

    # Before red the unreadable write could be product source, through sh -c
    # and through env -S alike.
    for command in (opaque, split):
        code, err = call(p, implementer, command=command)
        assert code == 10, (command, err)
        assert "no_red_evidence" in err, (command, err)
    # The test writer's before-red grant reads through the opacity.
    code, err = call(p, "vsdd-factory:test-writer", command=opaque)
    assert code == 0, err

    rule_projects.freeze(p.root, ["tests/test_sample.py"])
    rule_projects.red(p)

    # After red the unreadable write could be a frozen test.
    for command in (opaque, split):
        code, err = call(p, implementer, command=command)
        assert code == 10, (command, err)
        assert "red_test_frozen" in err, (command, err)
    # Read plainly, the same sed on unfrozen source passes as ever.
    code, err = call(p, implementer, command="sed -i 's/a/b/' src/lib.rs")
    assert code == 0, err
    # A read-only opaque segment is no write at all.
    code, err = call(p, implementer, command='sh -c "grep -c a src/lib.rs"')
    assert code == 0, err


def test_not_applied(tmp_path):    # A story whose kind has no red gate, a write outside any story worktree,
    # and a path in no class: the rule does not apply.
    p = rule_projects.story_project(tmp_path, kind="facade")
    assert call(p, "vsdd-factory:implementer", file_path=str(p.worktree / "src" / "lib.rs"))[0] == 0

    # A docs story has no red gate either (S4R-rules-26).
    docs = rule_projects.story_project(tmp_path, kind="docs")
    assert call(docs, "vsdd-factory:implementer", file_path=str(docs.worktree / "src" / "lib.rs"))[0] == 0

    payload = {
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "session_id": "sess-rules",
        "cwd": str(p.root),
        "tool_input": {"command": f"echo x > {p.root}/src/main.rs"},
    }
    code, _out, err = run_hook(payload, "gate", entries=(ROW,), env=base_env(p.root))
    assert code == 0, err

    unclassified = str(p.worktree / "notes" / "scratch.txt")
    assert call(p, "vsdd-factory:implementer", file_path=unclassified)[0] == 0

    # A spike worktree is exempt (the row's spike_exempt): a source-shaped
    # path under .worktrees/spike-probe passes. The case documents the
    # exemption's reach, it cannot fail by itself: a spike directory is no
    # story, so the rule is silent there with or without the exemption
    # (S4R-rules-26). The runner's skip itself is hooked-runner's to prove.
    spike = p.root / ".worktrees" / "spike-probe"
    (spike / "src").mkdir(parents=True, exist_ok=True)
    (spike / "src" / "probe.rs").write_text("fn main() {}\n", encoding="utf-8")
    assert call(p, "vsdd-factory:implementer", file_path=str(spike / "src" / "probe.rs"),
                cwd=spike)[0] == 0


def test_delegated_writers_are_the_edits_they_could_be(tmp_path):
    """A writer behind a wrapper, a find or a loop is the write it could be
    (step4-rules-fix-3-C1-9): after red it could change a frozen test, and
    before red it could make product source. The heredoc message that only
    names writers is no write."""
    p = rule_projects.story_project(tmp_path)
    implementer = "vsdd-factory:implementer"
    frozen_cmd = "sed -i 's/a/b/' tests/test_sample.py"

    before_red = (
        "flock lock.file sed -i 's/a/b/' src/lib.rs",
        "caffeinate -i sed -i 's/a/b/' src/lib.rs",
        "script -q /dev/null perl -pi -e 's/a/b/' src/lib.rs",
        "find src -exec perl -pi -e 's/a/b/' {} +",
    )
    for command in before_red:
        code, err = call(p, implementer, command=command)
        assert code == 10, (command, err)
        assert "no_red_evidence" in err, (command, err)

    rule_projects.freeze(p.root, ["tests/test_sample.py"])
    rule_projects.red(p, frozen=["tests/test_sample.py"])

    after_red = (
        "flock lock.file sed -i 's/a/b/' tests/test_sample.py",
        "caffeinate -i sed -i 's/a/b/' tests/test_sample.py",
        "script -q /dev/null perl -pi -e 's/a/b/' tests/test_sample.py",
        "find tests -exec sed -i 's/a/b/' {} +",
        "for f in tests/*.py; do sed -i 's/a/b/' \"$f\"; done",
        "F=tests/test_sample.py; sed -i 's/a/b/' $F",
        # through a read wrapper the parser does not enter
        "flock lock.file " + frozen_cmd,
    )
    for command in after_red:
        code, err = call(p, implementer, command=command)
        assert code == 10, (command, err)
        assert "red_test_frozen" in err, (command, err)
    # A writer word inside a commit message is message text, not a write.
    message = 'git commit -m "$(cat <<\'MSG\'\nteach rm and touch patience\nMSG)"'
    code, err = call(p, implementer, command=message)
    assert code == 0, (message, err)


def test_a_write_behind_a_cd_is_judged_in_an_unknown_place(tmp_path):
    """An opaque segment whose text holds a cd runs somewhere its own
    directory does not name: the write is judged against the stories of the
    governing project (step4-rules-fix-3-C1-9)."""
    p = rule_projects.story_project(tmp_path)
    implementer = "vsdd-factory:implementer"
    rule_projects.freeze(p.root, ["tests/test_sample.py"])
    rule_projects.red(p, frozen=["tests/test_sample.py"])

    for command in (
        f"(cd {p.worktree} && sed -i 's/a/b/' tests/test_sample.py)",
        f"sh -c 'cd {p.worktree} && sed -i s/a/b/ tests/test_sample.py'",
        # aimed outside the project, it is an unknown place still: the rule
        # cannot tell, and a cd-hidden write blocks
        f"(cd {p.root.parent / 'untouched'} && sed -i 's/a/b/' notes.txt)",
    ):
        code, err = call(p, implementer, command=command, cwd=p.root)
        assert code == 10, (command, err)
        assert "red_test_frozen" in err, (command, err)
    # The escapement is the plain, readable write: it is judged for what it
    # writes, and an unfrozen path passes.
    code, err = call(p, implementer, command="sed -i 's/a/b/' notes.txt")
    assert code == 0, err


def test_unreadable_kind_is_strict(tmp_path):
    p = rule_projects.story_project(tmp_path)
    # The story file is gone: the kind cannot be read, and the rule stays
    # strict (decision D4) instead of opening the gate.
    (p.root / ".factory" / "stories" / "E-9" / f"{p.story}.md").unlink()
    source = str(p.worktree / "src" / "lib.rs")
    code, err = call(p, "vsdd-factory:implementer", file_path=source)
    assert code == 10, err
    assert "no_red_evidence" in err, err
