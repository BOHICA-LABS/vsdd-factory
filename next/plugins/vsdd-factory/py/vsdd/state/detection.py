"""The values ``vsdd init`` detects, and the rendered ``factory.toml``
(state-core.md, decision 4).

At slice depth init writes every key of every fixed section: the detected
values where detection has an answer, the loader's strict defaults elsewhere.
The defaults come from ``vsdd.config.SPECS`` — decision 5 makes it the one key
list, so this module states no key or default a second time. Commands stay
empty and are listed in ``undetected`` — the survey fills them from an
approved toolchain file, never by guessing. ``delivery.protected_paths`` is
left out of the file: the loader computes it from ``[release]`` (the AU-T2-15
fix), and a written copy would freeze it at init time.
"""

from __future__ import annotations

import json
from pathlib import Path

from ..config import SPECS
from ..state.gitrun import git

__all__ = []

_RELEASE_BRANCHES = ("main", "master")
_INTEGRATION_BRANCHES = ("develop",)


def _branch_exists(root: Path, branch: str) -> bool:
    try:
        return bool(git(root, "show-ref", "--verify", f"refs/heads/{branch}", check=False).strip())
    except Exception:
        return False


def _current_branch(root: Path) -> str:
    try:
        return git(root, "symbolic-ref", "--short", "HEAD", check=False).strip() or "main"
    except Exception:
        return "main"


def _languages(root: Path) -> list[str]:
    languages: list[str] = []
    markers = (
        ("pyproject.toml", "python"),
        ("requirements.txt", "python"),
        ("Cargo.toml", "rust"),
        ("go.mod", "go"),
        ("package.json", "typescript"),
    )
    for marker, language in markers:
        if (root / marker).is_file() and language not in languages:
            languages.append(language)
    return languages


def _paths(root: Path) -> dict[str, list[str]]:
    paths: dict[str, list[str]] = {name: [] for name in (
        "tests", "harness", "ci", "repo_config", "manifests", "docs", "source"
    )}
    if (root / "tests").is_dir() or (root / "test").is_dir():
        paths["tests"].append("tests/**")
    for harness_dir in ("fuzz", "benches", "proofs"):
        if (root / harness_dir).is_dir():
            paths["harness"].append(f"{harness_dir}/**")
    if (root / ".github").is_dir():
        paths["ci"].append(".github/**")
    repo_configs = (".gitignore", ".editorconfig", ".flake8", "rustfmt.toml", "clippy.toml")
    paths["repo_config"] = [name for name in repo_configs if (root / name).is_file()]
    manifests = (
        "package.json", "package-lock.json", "Cargo.toml", "Cargo.lock",
        "pyproject.toml", "uv.lock", "poetry.lock", "go.mod", "go.sum",
    )
    paths["manifests"] = [name for name in manifests if (root / name).is_file()]
    if (root / "docs").is_dir():
        paths["docs"].append("docs/**")
    # Detection always puts README.md and CLAUDE.md in paths.docs (VSA-17).
    paths["docs"].extend(["README.md", "CLAUDE.md"])
    if (root / "src").is_dir():
        paths["source"].append("src/**")
    else:
        paths["source"].append("**")
    return paths


def _has_ci(root: Path) -> bool:
    return (
        (root / ".github" / "workflows").is_dir()
        or (root / ".gitlab-ci.yml").is_file()
        or (root / "Jenkinsfile").is_file()
        or (root / ".circleci").is_dir()
    )


def detect_values(root: Path) -> tuple[dict, list[str]]:
    """The detected values as ``dotted.key -> value``, and the undetected keys."""
    root = Path(root)
    detected: dict = {}
    try:
        has_remote = bool(git(root, "remote", check=False).strip())
    except Exception:
        has_remote = False
    detected["project.name"] = root.name
    detected["project.languages"] = _languages(root)
    integration = next(
        (b for b in _INTEGRATION_BRANCHES if _branch_exists(root, b)), _current_branch(root)
    )
    release = next((b for b in _RELEASE_BRANCHES if _branch_exists(root, b)), _current_branch(root))
    detected["branches.integration"] = integration
    detected["branches.release"] = release
    # The detected release branch is the one worth protecting (finding
    # S3R-23): a project releasing from master must not leave it exposed.
    detected["branches.protected"] = [release]
    detected["delivery.mode"] = "pr" if has_remote else "local"
    detected["delivery.ci"] = "required" if _has_ci(root) else "none"
    detected["state.push"] = has_remote
    for section, globs in _paths(root).items():
        detected[f"paths.{section}"] = globs
    undetected = [f"commands.{key}" for key in SPECS["commands"]]
    return detected, undetected


def _toml_value(value) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, str):
        return json.dumps(value)
    if isinstance(value, (int, float)):
        return repr(value)
    if isinstance(value, list):
        return "[" + ", ".join(_toml_value(item) for item in value) + "]"
    if isinstance(value, dict):
        return "{ " + ", ".join(f"{k} = {_toml_value(v)}" for k, v in value.items()) + " }"
    raise TypeError(f"cannot render {type(value).__name__} into factory.toml")


def render_config(root: Path, auto_allow: str | None = None) -> tuple[str, dict, list[str]]:
    """The whole ``factory.toml``: every fixed section of ``config.SPECS``,
    with the detected values laid over the loader's defaults.

    ``auto_allow`` is the ``--auto-allow`` answer, written into
    ``[hooks.rules.auto-allow]``. Returns ``(text, detected, undetected)``.
    ``delivery.protected_paths`` is not written: the loader computes it from
    ``[release]``, so a person's edit of ``release.changelog`` moves it.
    """
    detected, undetected = detect_values(root)

    lines: list[str] = ["schema = 1", ""]
    for section, keys in SPECS.items():
        if section == "project":
            continue  # written last, as the file's closing section
        lines.append(f"[{section}]")
        for key, spec in keys.items():
            dotted = f"{section}.{key}"
            if dotted == "delivery.protected_paths":
                continue  # the loader derives it from [release] (AU-T2-15)
            if dotted == "hooks.rules":
                continue  # a composite the loader fills; [hooks.rules.*] below
            value = detected.get(dotted, spec[0])
            lines.append(f"{key} = {_toml_value(value)}")
        lines.append("")
    lines.append("[project]")
    for key, spec in SPECS["project"].items():
        value = detected.get(f"project.{key}", spec[0])
        lines.append(f"{key} = {_toml_value(value)}")
    lines.append("")
    if auto_allow is not None:
        lines.append("[hooks.rules.auto-allow]")
        lines.append(f"scope = {_toml_value(auto_allow)}")
        lines.append("")
    return "\n".join(lines), detected, undetected
