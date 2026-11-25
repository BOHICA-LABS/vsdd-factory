"""Project detection: the governing project, the state directory, worktrees,
story and spike detection (hooks.md 2.4, 2.8; blueprint 4.7).

Purely file-backed: no subprocess, no network. Git facts that need more than
reading ``.git`` files belong to the hook layer's ``gitfs`` (H1), which builds
on :func:`read_branch` here.

Resolution rules the designs fix:

- the project root is the nearest ancestor directory holding
  ``.factory/factory.toml`` (state.md 2.12: the file exists if and only if the
  project is initialised for 1.0). :func:`find_project_root` is the Python twin
  of ``py/find-root.sh`` (hooks.md 2.8) and follows the same rules, so the
  shell's session project and the Python governing project cannot disagree:
  the walk stops below ``$HOME`` and ``/`` — a stray ``~/.factory`` adopts
  nothing — a linked worktree's ``.git`` pointer pulls its main checkout into
  the walk, and no symlink is followed, so a root compares equal to the
  ``VSDD_FACTORY_ROOT`` the launcher passes,
- the state directory is ``<root>/.factory``, exactly one per project and
  never present inside a story or spike worktree,
- worktrees live at ``.worktrees/<id>/`` for a story, ``.worktrees/_integration/``
  for wave and release gates (``_integration/<repo>/`` per member repository in
  a hub), and ``.worktrees/spike-<name>/`` for a named spike (blueprint 4.7).
  A story id carries the grammar of blueprint 4.8 (``S-9.01``): any other
  non-dot directory under ``.worktrees/`` is not a story,
- a spike is recognised two ways (hooks.md ``in_spike``): the worktree name
  starts with ``spike-`` or the branch is on ``spike/*`` — which is what makes
  a quick spike in a platform worktree (``.claude/worktrees/<id>``) a spike,
  since :func:`detect` reads the branch of the worktree that contains the path.
"""

from __future__ import annotations

import os
import re
from dataclasses import dataclass, field
from pathlib import Path

from .config import ConfigError, loads as _load_config

__all__ = [
    "FACTORY_DIR",
    "CONFIG_NAME",
    "Worktree",
    "Project",
    "find_project_root",
    "find_state_dir",
    "is_initialised",
    "has_legacy_state",
    "classify_worktree",
    "containing_worktree",
    "read_branch",
    "main_checkout",
    "story_branch_id",
    "path_class",
]

FACTORY_DIR = ".factory"
CONFIG_NAME = "factory.toml"

_INTEGRATION = "_integration"
_SPIKE_PREFIX = "spike-"
_BRANCH_REF_RE = re.compile(r"ref: (refs/heads/(.+))")

# blueprint 4.8 story grammar: S-<major>.<two digits>
STORY_GRAMMAR = re.compile(r"S-\d+\.\d{2}")

_DEFAULT_STORY_PATTERN = "story/{id}"
_DEFAULT_SPIKE_PATTERN = "spike/{name}"


def _pattern_regex(pattern: str) -> re.Pattern:
    """``story/{id}`` becomes ``^story/(?P<id>.+)$``; a literal fallback when
    the pattern carries no placeholder."""
    head, _, _tail = pattern.partition("{")
    if _tail == "" and "{" not in pattern:
        return re.compile(re.escape(pattern))
    name = "id" if "{id}" in pattern else "name"
    prefix = pattern.split("{", 1)[0]
    return re.compile(re.escape(prefix) + f"(?P<{name}>.+)$")


def _branch_patterns(root: Path) -> tuple[re.Pattern, re.Pattern]:
    """The ``[branches]`` story and spike patterns of the project, as regexes.

    Falls back to the design defaults when the configuration file does not
    parse — a broken configuration relaxes nothing here, and its errors are
    the survey's to report.
    """
    story, spike = _DEFAULT_STORY_PATTERN, _DEFAULT_SPIKE_PATTERN
    try:
        text = (root / FACTORY_DIR / CONFIG_NAME).read_text(encoding="utf-8")
        branches = _load_config(text, root / FACTORY_DIR / CONFIG_NAME).get("branches")
        story = branches.get("story") or story
        spike = branches.get("spike") or spike
    except (OSError, ConfigError):
        pass
    return _pattern_regex(story), _pattern_regex(spike)


def find_state_dir(start: Path | str) -> Path | None:
    """Nearest ancestor directory (or ``start`` itself) holding ``.factory/``."""
    current = Path(os.path.abspath(start))
    if current.is_file():
        current = current.parent
    while True:
        if (current / FACTORY_DIR).is_dir():
            return current / FACTORY_DIR
        parent = current.parent
        if parent == current:
            return None
        current = parent


def is_initialised(state_dir: Path) -> bool:
    """A project is initialised for 1.0 iff ``factory.toml`` exists."""
    return (state_dir / CONFIG_NAME).is_file()


def _worktree_main(path: Path) -> Path | None:
    """The main checkout when ``path`` carries a linked-worktree ``.git`` file."""
    git_path = path / ".git"
    if not git_path.is_file():
        return None
    try:
        content = git_path.read_text(encoding="utf-8").strip()
    except OSError:
        return None
    if not content.startswith("gitdir: ") or ".git/worktrees/" not in content:
        return None
    gitdir = content[len("gitdir: ") :]
    main = Path(gitdir).parts[: -len(Path(".git/worktrees").parts) - 1]
    if not main:
        return None
    if Path(*main).is_absolute():
        return Path(os.path.normpath(Path(*main)))
    return Path(os.path.normpath(path / Path(*main)))


def find_project_root(start: Path | str, home: Path | str | None = None) -> Path | None:
    """Nearest ancestor holding an initialised ``.factory/factory.toml``.

    The Python twin of ``py/find-root.sh`` (hooks.md 2.8): the walk stops below
    ``home`` (``$HOME``, default) and the filesystem root, so a stray
    ``~/.factory`` adopts nothing; at every directory whose ``.git`` is a
    linked-worktree pointer, the main checkout is tested too; and no symlink is
    resolved, so the returned root compares equal to the string path the
    launcher hands over in ``VSDD_FACTORY_ROOT``.

    ``None`` outside a factory project: the hooks stay silent there
    (decision D24). A ``home`` of ``None`` means ``$HOME`` (finding SL1-48):
    the walk always stops below the person's home directory unless a caller
    passes an explicit ``home`` of its own.
    """
    current = Path(os.path.abspath(start))
    if current.is_file():
        current = current.parent
    if home is None:
        home = os.environ.get("HOME") or None  # an empty HOME is unset
    stop_home = os.path.abspath(str(home)) if home is not None else None
    while True:
        # Home and the filesystem root are walked up to, never used as roots.
        if current == Path("/") or (stop_home is not None and current == Path(stop_home)):
            return None
        candidate = current / FACTORY_DIR
        if candidate.is_dir() and is_initialised(candidate):
            return current
        main = _worktree_main(current)
        if main is not None:
            candidate = main / FACTORY_DIR
            if candidate.is_dir() and is_initialised(candidate):
                return main
        parent = current.parent
        if parent == current:
            return None
        current = parent


def has_legacy_state(root: Path | str) -> bool:
    """Pre-1.0 state: a ``.factory/`` with ``STATE.md`` and no ``factory.toml``."""
    state_dir = Path(root) / FACTORY_DIR
    return state_dir.is_dir() and (state_dir / "STATE.md").is_file() and not is_initialised(state_dir)


@dataclass(frozen=True)
class Worktree:
    """One classified worktree entry.

    ``kind`` is ``"story"``, ``"spike"`` or ``"integration"``; ``id`` the story
    id, spike name, or ``""`` for the integration worktree; ``repo`` names the
    member repository of a hub's ``_integration/<repo>/`` worktree.
    """

    kind: str
    id: str
    path: Path
    repo: str = field(default="")


def classify_worktree(name: str) -> Worktree | None:
    """Classify a ``.worktrees/<name>`` entry by its directory name.

    ``_integration`` is the wave and release worktree and ``spike-<name>`` a
    named spike; a story id must carry the blueprint 4.8 grammar
    (``S-9.01``). Any other name classifies as None: ``tmp`` is not a story,
    and neither is a bare ``spike-`` with no name.
    """
    if name == _INTEGRATION:
        return Worktree("integration", "", Path(".worktrees") / name)
    if name.startswith(_SPIKE_PREFIX) and len(name) > len(_SPIKE_PREFIX):
        return Worktree("spike", name[len(_SPIKE_PREFIX) :], Path(".worktrees") / name)
    if STORY_GRAMMAR.fullmatch(name):
        return Worktree("story", name, Path(".worktrees") / name)
    return None


def containing_worktree(path: Path | str, root: Path | str | None = None) -> Worktree | None:
    """The worktree under ``.worktrees`` that contains ``path``.

    Anchored on the project root's own ``.worktrees`` directory when ``root``
    is given, so a vendored ``.worktrees`` deeper inside a story does not
    classify; without ``root``, the nearest ``.worktrees`` above the path is
    used. ``_integration/<repo>/`` carries the repository name.
    """
    current = Path(os.path.abspath(path))
    if current.is_file():
        current = current.parent
    if root is not None:
        anchor = Path(os.path.abspath(root)) / ".worktrees"
    else:
        anchor = _nearest_worktrees_parent(current) / ".worktrees"
    try:
        relative = current.relative_to(anchor)
    except ValueError:
        return None
    parts = relative.parts
    if not parts:
        return None
    worktree = classify_worktree(parts[0])
    if worktree is None:
        return None
    holder = anchor / parts[0]
    repo = ""
    if worktree.kind == "integration" and len(parts) >= 2 and not parts[1].startswith("."):
        # _integration/<repo>/ scopes the worktree to one member repository.
        repo = parts[1]
        holder = holder / repo
    return Worktree(worktree.kind, worktree.id, holder, repo)


def _nearest_worktrees_parent(current: Path) -> Path:
    parts = current.parts
    if ".worktrees" not in parts:
        return current
    at = len(parts) - 1 - parts[::-1].index(".worktrees")
    return Path(*parts[:at])


def read_branch(repo: Path | str) -> str | None:
    """The checked-out branch of a checkout or linked worktree, or None.

    Reads ``HEAD`` through the ``.git`` pointer file of a linked worktree
    (``gitdir: <path>``) or the ``.git`` directory itself. Detached state
    returns the commit SHA, matching git's own resolution.
    """
    base = Path(repo)
    git_path = base / ".git"
    try:
        if git_path.is_file():
            content = git_path.read_text(encoding="utf-8").strip()
            if not content.startswith("gitdir:"):
                return None
            gitdir = Path(content[len("gitdir:") :].strip())
            if not gitdir.is_absolute():
                gitdir = base / gitdir
        elif git_path.is_dir():
            gitdir = git_path
        else:
            return None
        head = (gitdir / "HEAD").read_text(encoding="utf-8").strip()
    except OSError:
        return None
    match = _BRANCH_REF_RE.fullmatch(head)
    if match:
        return match.group(2)
    return head or None


def main_checkout(worktree: Path | str) -> Path | None:
    """The main checkout of a linked worktree, or None when ``worktree`` is one.

    Reads the ``.git`` pointer file: ``gitdir: <main>/.git/worktrees/<name>``
    resolves to ``<main>``, normalised, so a relative gitdir from git 2.48
    (``worktree.useRelativePaths``) compares equal to an absolute one.
    Events and locks live with the main project, which is why worktree calls
    resolve here first (hooks.md 2.12).
    """
    base = Path(worktree)
    git_path = base / ".git"
    if not git_path.is_file():
        return None
    try:
        content = git_path.read_text(encoding="utf-8").strip()
    except OSError:
        return None
    if not content.startswith("gitdir:"):
        return None
    gitdir = Path(content[len("gitdir:") :].strip())
    if not gitdir.is_absolute():
        gitdir = base / gitdir
    gitdir = Path(os.path.normpath(gitdir))
    # <main>/.git/worktrees/<name> -> <main>
    if len(gitdir.parents) >= 3 and gitdir.parent.name == "worktrees":
        return gitdir.parents[2]
    return None


def story_branch_id(branch: str, pattern: re.Pattern) -> str | None:
    """The story id a checked-out story branch holds, or None."""
    match = pattern.match(branch)
    if not match:
        return None
    return match.group("id" if "id" in pattern.groupindex else "name")


@dataclass(frozen=True)
class Project:
    """The governing project for a path: what ``Ctx.project`` holds."""

    root: Path
    state_dir: Path
    worktree: Worktree | None
    branch: str | None = None

    @property
    def in_spike(self) -> bool:
        return self.worktree is not None and self.worktree.kind == "spike"


def _branch_home(path: Path) -> Path | None:
    """The nearest directory of ``path`` that is a linked git worktree."""
    current = Path(os.path.abspath(path))
    if current.is_file():
        current = current.parent
    while True:
        if (current / ".git").is_file():
            return current
        parent = current.parent
        if parent == current:
            return None
        current = parent


def detect(path: Path | str) -> Project | None:
    """Resolve the governing project for ``path`` in one call.

    Walks up from the path to the nearest initialised state directory
    (hooks.md ``Ctx.project``); classifies the containing worktree, and reads
    the branch of the worktree that contains the path, so a spike is
    recognised from its branch (``spike/*``) even outside ``.worktrees/`` —
    in a platform worktree (``.claude/worktrees/<id>``) too.
    """
    root = find_project_root(path)
    if root is None:
        return None
    worktree = containing_worktree(path, root=root)
    branch = None
    branch_home = _branch_home(Path(os.path.abspath(path)))
    if branch_home is not None:
        branch = read_branch(branch_home)
    story_pattern, spike_pattern = _branch_patterns(root)

    kind = worktree.kind if worktree else None
    identifier = worktree.id if worktree else ""
    location = worktree.path if worktree else None
    spike_branch = story_branch_id(branch, spike_pattern) if branch else None
    story_branch = story_branch_id(branch, story_pattern) if branch else None
    if kind == "story" and not STORY_GRAMMAR.fullmatch(identifier):
        kind, identifier, location = None, "", None

    if kind is None and branch is not None:
        if spike_branch is not None:
            kind, identifier = "spike", spike_branch
            location = branch_home
        elif story_branch is not None and STORY_GRAMMAR.fullmatch(story_branch):
            kind, identifier = "story", story_branch
            location = branch_home
    elif kind == "story" and story_branch is not None and STORY_GRAMMAR.fullmatch(story_branch):
        identifier = story_branch

    classified = Worktree(kind, identifier, location) if kind else None
    return Project(
        root=root,
        state_dir=root / FACTORY_DIR,
        worktree=classified,
        branch=branch,
    )


def _holding_worktree_root(path: Path) -> Path | None:
    """The nearest ancestor of ``path`` that is a git worktree root: the
    first directory upward whose ``.git`` (file or directory) exists."""
    current = Path(os.path.abspath(path))
    if current.is_file():
        current = current.parent
    while True:
        if (current / ".git").exists():
            return current
        parent = current.parent
        if parent == current:
            return None
        current = parent


def path_class(path, *, root=None, config=None) -> str | None:
    """The product class of an absolute path, by ``[_product].precedence``
    (state-core.md 3.4).

    The first class in the precedence whose ``from_config`` globs match the
    path relative to the root of the git worktree holding it. ``None`` when
    no glob matches, when the path is in no project, or when it is under
    ``.factory/``. ``root`` and ``config`` spare a caller that has them the
    search and the load (the hook passes both); when ``path_class`` loads the
    file itself, an invalid one raises ``ConfigError``.
    """
    from .config import ConfigError  # re-exported name, kept close to its use
    from .config import load as load_config
    from .globs import glob_regex
    from .policy import load as load_policy

    del ConfigError  # only named for the contract; loads() raises it
    path = Path(os.path.abspath(path))
    if root is None:
        root = find_project_root(path, home=os.environ.get("HOME"))
        if root is None:
            return None
    root = Path(os.path.abspath(root))
    state_dir = root / FACTORY_DIR
    try:
        relative_to_root = path.relative_to(root)
    except ValueError:
        return None
    if relative_to_root.parts[:1] == (FACTORY_DIR,):
        return None
    if config is None:
        config = load_config(state_dir / CONFIG_NAME)
    data = getattr(config, "data", config)

    worktree_root = _holding_worktree_root(path) or root
    try:
        relative = path.relative_to(worktree_root)
    except ValueError:
        return None
    posix = relative.as_posix()
    if not posix or posix == ".":
        return None

    classes = load_policy("classes")
    for name in classes["_product"]["precedence"]:
        from_config = classes.get(name, {}).get("from_config", "")
        section, _, key = from_config.partition(".")
        globs = (data.get(section) or {}).get(key, []) if isinstance(data, dict) else []
        for glob in globs:
            if glob_regex(str(glob)).fullmatch(posix):
                return name
    return None
