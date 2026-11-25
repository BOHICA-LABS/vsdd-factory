"""The grounding check (plan rules.md 3.5; PL4-1): every reason, the exemptions,
the path escapes, and the shape tolerance."""

from __future__ import annotations

import os
import stat
from pathlib import Path

from vsdd.grounding import ground


def make_tree(tmp_path: Path) -> tuple[Path, Path]:
    tree = tmp_path / "tree"
    (tree / "src").mkdir(parents=True)
    (tree / "src" / "main.rs").write_text(
        "\n".join(
            [
                "pub fn one() -> usize {",  # line 1
                "    let half = 1;",  #        line 2
                "    half + 1",  #             line 3
                "}",  #                        line 4
                "",  #                         line 5
                "pub fn far() -> usize {",  #  line 6
                "    4",  #                    line 7
                "}",  #                        line 8
            ]
        ),
        encoding="utf-8",
    )
    (tree / "src" / "spacing.md").write_text("the   spaced   text\n", encoding="utf-8")
    return tree, tree


def finding(**location) -> dict:
    return {"key": "code:x", "category": "code", "location": location}


def test_ground_reasons(tmp_path):
    tree, project_root = make_tree(tmp_path)

    # Grounded: path, lines, quote.
    assert ground(finding(path="src/main.rs", line_start=2, line_end=3, quote="half"),
                  tree=tree, project_root=project_root) == ""
    # Whitespace in the quote is normalised.
    assert ground(finding(path="src/spacing.md", quote="the spaced text"),
                  tree=tree, project_root=project_root) == ""
    # File not found.
    assert ground(finding(path="src/missing.rs"), tree=tree, project_root=project_root) == (
        "file not found: src/missing.rs"
    )
    # Lines outside the file, with the line count named.
    assert ground(finding(path="src/main.rs", line_start=2, line_end=9),
                  tree=tree, project_root=project_root) == (
        "lines 2-9 are outside src/main.rs (8 lines)"
    )
    # Zero lines: an empty file refuses any line claim.
    (tree / "src" / "empty.txt").write_text("", encoding="utf-8")
    assert ground(finding(path="src/empty.txt", line_start=1),
                  tree=tree, project_root=project_root) == (
        "lines 1-1 are outside src/empty.txt (0 lines)"
    )
    # Quote not found: the range widened by three lines misses line 6.
    assert ground(finding(path="src/main.rs", line_start=2, line_end=2, quote="pub fn far"),
                  tree=tree, project_root=project_root) == "quote not found in src/main.rs"
    # And the whole file, with no line given at all.
    assert ground(finding(path="src/main.rs", quote="no such text"),
                  tree=tree, project_root=project_root) == "quote not found in src/main.rs"


def test_three_line_window(tmp_path):
    tree, project_root = make_tree(tmp_path)
    # Line 1 claims a quote from line 2: inside the widened window (1-4).
    assert ground(finding(path="src/main.rs", line_start=1, line_end=1, quote="let half"),
                  tree=tree, project_root=project_root) == ""
    # Line 4 claims a quote from line 6: three lines of slack reach it.
    assert ground(finding(path="src/main.rs", line_start=4, line_end=4, quote="pub fn far"),
                  tree=tree, project_root=project_root) == ""
    # Line 1 with a quote from line 6 is refused: the window is three lines,
    # not the file.
    assert ground(finding(path="src/main.rs", line_start=1, line_end=1, quote="pub fn far"),
                  tree=tree, project_root=project_root) == "quote not found in src/main.rs"
    # A single-bound finding seeks its window too.
    assert ground(finding(path="src/main.rs", line_start=6, quote="    4"),
                  tree=tree, project_root=project_root) == ""


def test_path_escape_is_refused(tmp_path):
    tree, project_root = make_tree(tmp_path)
    outside = tmp_path / "outside.txt"
    outside.write_text("secret\n", encoding="utf-8")

    # A climbing path…
    assert ground(finding(path="../outside.txt"), tree=tree, project_root=project_root) == (
        "path is outside the reviewed tree: ../outside.txt"
    )
    # …an absolute path…
    assert ground(finding(path=str(outside)), tree=tree, project_root=project_root) == (
        f"path is outside the reviewed tree: {outside}"
    )
    # …and a link out of the tree.
    link = tree / "src" / "escape.rs"
    link.symlink_to(outside)
    assert ground(finding(path="src/escape.rs"), tree=tree, project_root=project_root) == (
        "path is outside the reviewed tree: src/escape.rs"
    )


def test_factory_paths_use_the_project_root(tmp_path):
    tree = tmp_path / "tree"
    tree.mkdir()
    (tree / "worktree-story-file.txt").write_text("story\n", encoding="utf-8")
    project_root = tmp_path / "project"
    (project_root / ".factory" / "stories" / "E-9").mkdir(parents=True)
    story = project_root / ".factory" / "stories" / "E-9" / "S-9.01.md"
    story.write_text("---\nid: S-9.01\n---\nThe contract clause.\n", encoding="utf-8")

    # A .factory path resolves against the project root, though the tree has
    # no .factory of its own.
    assert ground(finding(path=".factory/stories/E-9/S-9.01.md", quote="The contract clause."),
                  tree=tree, project_root=project_root) == ""
    # A product path still resolves against the tree.
    assert ground(finding(path="worktree-story-file.txt", quote="story"),
                  tree=tree, project_root=project_root) == ""


def test_holdout_and_unplaced_are_exempt(tmp_path):
    tree, project_root = make_tree(tmp_path)
    # Category holdout: no location is read at all.
    assert ground({"category": "holdout", "location": {"path": "../outside.txt"}},
                  tree=tree, project_root=project_root) == ""
    # No location, or no path in it: the tool holds these advisory.
    assert ground({"category": "code"}, tree=tree, project_root=project_root) == ""
    assert ground({"category": "code", "location": {}}, tree=tree, project_root=project_root) == ""
    assert ground({"category": "code", "location": {"path": ""}},
                  tree=tree, project_root=project_root) == ""


def test_unreadable_file_is_reported(tmp_path):
    """A cited file that exists but cannot be read does not ground by the
    catch-all: the reason says so (S4R-rules-20)."""
    tree, project_root = make_tree(tmp_path)
    locked = tree / "src" / "locked.rs"
    locked.write_text("secret\n", encoding="utf-8")
    mode = locked.stat().st_mode
    os.chmod(locked, mode & ~stat.S_IRWXU & ~stat.S_IRWXG & ~stat.S_IRWXO)
    try:
        assert ground(finding(path="src/locked.rs"), tree=tree, project_root=project_root) == (
            "file cannot be read: src/locked.rs"
        )
    finally:
        os.chmod(locked, mode)


def test_factory_dotdot_paths_use_the_tree(tmp_path):
    """A ``.factory/..`` path climbs out of the state directory, so it is the
    tree's file; one that stays under ``.factory`` is the project's
    (S4R-rules-20)."""
    tree = tmp_path / "tree"
    (tree / "src").mkdir(parents=True)
    (tree / "src" / "main.rs").write_text("one\n", encoding="utf-8")
    project_root = tmp_path / "project"
    (project_root / ".factory").mkdir(parents=True)
    (project_root / ".factory" / "notes.md").write_text("kept\n", encoding="utf-8")
    (project_root / "src").mkdir()
    (project_root / "src" / "main.rs").write_text("wrong tree\n", encoding="utf-8")

    assert ground(finding(path=".factory/../src/main.rs", quote="one"),
                  tree=tree, project_root=project_root) == ""
    # And the state directory's own files, dot segments and all.
    assert ground(finding(path=".factory/./notes.md", quote="kept"),
                  tree=tree, project_root=project_root) == ""


def test_malformed_finding_never_raises(tmp_path):
    tree, project_root = make_tree(tmp_path)
    for shape in (
        None,
        "a string",
        7,
        [finding(path="src/main.rs")],
        {"location": "not a table"},
        {"location": {"path": 7}},
        {"location": {"path": "src/main.rs", "line_start": "2", "line_end": None}},
        {"location": {"path": "src/main.rs", "line_start": True}},
        {"location": {"path": "src/main.rs", "quote": 99}},
        {"location": {"path": "src/main.rs/", "line_start": -5, "line_end": 0}},
    ):
        ground(shape, tree=tree, project_root=project_root)  # must not raise
