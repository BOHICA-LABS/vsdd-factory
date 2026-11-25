"""The ``product`` option of the story-ready project builder (PL6-7).

A directory product is copied in and committed on the integration branch
before the story is admitted, but the project keeps its own configuration:
the product's ``.factory`` is not copied, and its ``[paths]`` table — which
describes the layout the copy recreates — is applied instead of the copy
blanking every path class with a pattern per top-level name. Nothing else is
taken from the product's ``factory.toml``: ``[state]`` and ``[delivery]``
stay the project's own.
"""

from __future__ import annotations

import subprocess
import tomllib
from pathlib import Path

from .story_ready import SESSION, make_project

from vsdd.config import load as load_config

SAMPLE = Path(__file__).resolve().parents[1] / "e2e" / "sample-project"

#: The sample's configuration, read straight from its file: the fixed side
#: the assertions below discriminate against.
SAMPLE_CONFIG = tomllib.loads((SAMPLE / ".factory" / "factory.toml").read_text(encoding="utf-8"))


def _config(root: Path):
    return load_config(root / ".factory" / "factory.toml")


def _git(root: Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=root, text=True)


def test_product_directory_keeps_the_project_configuration(tmp_path):
    root = make_project(tmp_path, product=SAMPLE)

    # The discriminator is real: the sample's [delivery].mode is "pr" and its
    # [state].push is true, so these are the project's own values, not ones
    # the sample happens to agree with.
    assert SAMPLE_CONFIG["delivery"]["mode"] == "pr"
    assert SAMPLE_CONFIG["state"]["push"] is True
    config = _config(root)
    assert config.get("delivery", "mode") == "local"
    assert config.get("state", "push") is False
    assert config.command("test") == "python -m pytest -q"


def test_product_directory_sets_paths_from_the_sample(tmp_path):
    root = make_project(tmp_path, product=SAMPLE)

    # The whole table, every class of it — not only the two keys that happen
    # to be populated: repo_config must come along and docs must not be left
    # at the init default.
    paths = _config(root).get("paths")
    assert paths == SAMPLE_CONFIG["paths"]
    # No pattern may classify a path as both a test and a source.
    assert not set(paths["tests"]) & set(paths["source"])


def test_product_is_committed_on_the_integration_branch(tmp_path):
    root = make_project(tmp_path, product=SAMPLE)

    assert _git(root, "rev-parse", "--abbrev-ref", "HEAD").strip() == "develop"
    commit = _git(root, "log", "--format=%H", "--grep=sample product", "-1").strip()
    changed = _git(root, "diff-tree", "--no-commit-id", "--name-only", "-r", commit).splitlines()
    assert "tally/stats.py" in changed
    assert "tools/junit_runner.py" in changed
    # The project's own configuration, not the sample's: make_project set
    # commands.test before the product was copied in, and the copy takes
    # only the sample's [paths].
    config = load_config(root / ".factory" / "factory.toml")
    assert config.command("test") == "python -m pytest -q"
    assert config.get("delivery", "mode") == "local"


def test_product_without_factory_toml_sources_directories_only(tmp_path):
    product = tmp_path / "product"
    (product / "lib").mkdir(parents=True)
    (product / "lib" / "impl.py").write_text("value = 1\n", encoding="utf-8")
    (product / "NOTES.txt").write_text("a top-level file\n", encoding="utf-8")

    root = make_project(tmp_path, product=product)

    assert _config(root).get("paths", "source") == ["lib/**"]
    assert (root / "lib" / "impl.py").is_file()
    assert (root / "NOTES.txt").is_file()


def test_product_default_story_still_admits(tmp_path):
    """The option composes with the rest of the builder: the sample story is
    admitted through the usual entry (session constant unchanged)."""
    from vsdd.state.locks import lock_scope, run_begin

    root = make_project(tmp_path, product=SAMPLE)
    config = _config(root)
    assert config.get("delivery", "mode") == "local"
    begun = run_begin(
        root,
        scope=lock_scope(root),
        workflow="story",
        subject="S-9.01",
        session=SESSION,
        cycle="c-1",
    )
    assert begun["run"]["id"]
