"""The digest (plan records.md 7, test_digest): a pinned value; order-free;
content-bound; a missing file refuses; brief and guidance are left out."""

from __future__ import annotations

import hashlib

import pytest

from .state_fixtures import make_project, spec_file
from vsdd.state import digests
from vsdd.state.store import StoreRefused


def test_digest(tmp_path):
    (tmp_path / ".factory" / "x").mkdir(parents=True)
    (tmp_path / ".factory" / "x" / "a.txt").write_bytes(b"hello\n")
    (tmp_path / ".factory" / "x" / "b.txt").write_bytes(b"world\n")
    paths = [".factory/x/a.txt", ".factory/x/b.txt"]

    # The formula pinned (the topic defines it once): SHA-256 of the lines
    # "<path>\t<sha256 of the file's bytes>\n", sorted by path, cut to 16 hex
    # characters. The literal below was computed by hand for these two files.
    lines = (
        ".factory/x/a.txt\t" + hashlib.sha256(b"hello\n").hexdigest() + "\n"
        ".factory/x/b.txt\t" + hashlib.sha256(b"world\n").hexdigest() + "\n"
    )
    pinned = hashlib.sha256(lines.encode("utf-8")).hexdigest()[:16]

    actual = digests.digest(paths, root=tmp_path)
    assert actual == pinned
    # Order-free and content-bound.
    assert digests.digest(reversed(paths), root=tmp_path) == pinned
    assert digests.digest([".factory/x/a.txt"], root=tmp_path) != actual
    (tmp_path / ".factory" / "x" / "a.txt").write_bytes(b"changed\n")
    assert digests.digest([".factory/x/a.txt"], root=tmp_path) != digests.digest(
        [".factory/x/b.txt"], root=tmp_path
    )

    with pytest.raises(StoreRefused) as exc:
        digests.digest([".factory/x/absent.txt"], root=tmp_path)
    assert exc.value.code == "digest_missing_file"


def test_spec_file_set(tmp_path):
    root = make_project(tmp_path)
    spec_file(root, ".factory/specs/product/prd.md", "# PRD\n")
    spec_file(root, ".factory/specs/product/brief.md", "# Brief\n")
    spec_file(root, ".factory/specs/architecture/SS-01.md", "# Architecture\n")
    spec_file(root, ".factory/specs/architecture/conventions.md", "# Conventions\n")
    spec_file(root, ".factory/specs/architecture/review-rules.md", "# Review rules\n")
    spec_file(root, ".factory/specs/ux/overview.md", "# UX\n")

    paths = digests.spec_file_set(root)
    assert ".factory/specs/product/prd.md" in paths
    assert ".factory/specs/architecture/SS-01.md" in paths
    assert ".factory/specs/ux/overview.md" in paths
    # Brief and guidance are left out: their own locks and decisions.
    assert ".factory/specs/product/brief.md" not in paths
    assert ".factory/specs/architecture/conventions.md" not in paths
    assert ".factory/specs/architecture/review-rules.md" not in paths

    spec_digest, spec_paths = digests.spec_digest(root)
    assert spec_digest == digests.digest(paths, root=root)
    assert spec_paths == paths
