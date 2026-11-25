"""The rule model: outcome types, the rule declaration and ``Ctx``
([[topic:rule-model]]).

HD1: the registry row is the rule's one declaration; there is no decorator. An
import crash must become the rule's declared failure, which needs ``kind`` and
``fail`` before the module loads, so the declaration lives as data in
:mod:`vsdd.hooks.registry` and the rule module holds only ``check``.

``Ctx`` is built once per call; the derived fields are computed on first use
and cached. Fields the design does not settle are decided in the step 3 plan
(hook-runner.md 3.2): payload fields, project subclassing, target/where
resolution, story and spike detection, and the row lookup. The one field of
H4 (``new_content``) still raises :class:`ContextUnavailable` until its
package builds; ``classes`` of the second part of H1 is built
(``paths.classes_of``).
"""

from __future__ import annotations

import importlib
import os
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable, Mapping

from .. import config as _config
from .. import events as _events
from .. import project as _project

__all__ = [
    "Block",
    "Advise",
    "Context",
    "Permit",
    "Rewrite",
    "RuleConfig",
    "RuleEntry",
    "Ctx",
    "ContextUnavailable",
    "EVENT_RULE_BLOCK",
    "EVENT_RULE_ADVISE",
    "EVENT_RULE_WOULD_BLOCK",
    "EVENT_RULE_ERROR",
    "EVENT_RULE_TIMEOUT",
    "EVENT_RUNNER_CALL",
    "EVENT_RUNNER_SLOW",
    "EVENT_REWRITE_DISCARDED",
    "EVENT_CONFIG_INVALID",
    "EVENT_POLICY_INVALID",
    "EVENT_LOCK_RELEASED",
    "EVENT_RUNNER_LIVENESS_ERROR",
]


# --- the event types the runner writes --------------------------------------
#
# One constant each, so the writer's spellings hang from one definition
# (tables/event-types.toml is the machine copy of the table; plan 3.1's note
# on L2-catchup-fix-3).

EVENT_RULE_BLOCK = "rule.block"              # a rule's Block is emitted
EVENT_RULE_ADVISE = "rule.advise"            # a rule's Advise is delivered
EVENT_RULE_WOULD_BLOCK = "rule.would_block"  # a Block relaxed at advise strength
EVENT_RULE_ERROR = "rule.error"              # a rule crashed
EVENT_RULE_TIMEOUT = "rule.timeout"          # a rule ran out of time
EVENT_RUNNER_CALL = "runner.call"            # every call, when DEBUG=1
EVENT_RUNNER_SLOW = "runner.slow"            # a call took over 300 ms
EVENT_REWRITE_DISCARDED = "rewrite.discarded"  # a second Rewrite of one call, dropped
EVENT_CONFIG_INVALID = "config.invalid"      # the project configuration is invalid
EVENT_POLICY_INVALID = "policy.invalid"      # a policy table is invalid
EVENT_LOCK_RELEASED = "lock.released"        # a run-lock entry released at session end
EVENT_RUNNER_LIVENESS_ERROR = "runner.liveness_error"  # the lock liveness duty faulted


# --- outcomes -------------------------------------------------------------

@dataclass(frozen=True)
class Block:
    """Stop the action. ``reason`` says what was attempted and why it is not
    allowed; ``fix`` names the correct route."""

    code: str
    reason: str
    fix: str = ""


@dataclass(frozen=True)
class Advise:
    """Allow, and tell the model something."""

    code: str
    message: str


@dataclass(frozen=True)
class Context:
    """Add context (briefing, compaction instructions); ``system`` becomes the
    platform's ``systemMessage``."""

    text: str
    system: str = ""


@dataclass(frozen=True)
class Permit:
    """``PreToolUse`` only: skip the permission prompt for this call."""

    reason: str


@dataclass(frozen=True)
class Rewrite:
    """Return ``updatedInput`` for the call: the whole replacement input.

    Taken once per call from a rule whose row sets ``rewrites`` (HD10), and
    from the runner itself for a ``vsdd`` Bash call, to add the marker's key
    to the command ([[record:caller-marker]]).
    """

    tool_input: dict
    reason: str


@dataclass(frozen=True)
class RuleConfig:
    """What the project's ``[hooks.rules.<id>]`` table configures. ``strength``
    is ``block``, ``advise`` or ``off``; a locked rule always gets ``block``.
    ``options`` holds the rule's other keys."""

    strength: str = "block"
    options: Mapping[str, object] = field(default_factory=dict)


class ContextUnavailable(Exception):
    """A context field whose owning package has not built it (``new_content``
    of H4). Reading it inside a rule is that rule's crash."""


@dataclass(frozen=True)
class RuleEntry:
    """One rule's declaration (HD1). ``on`` maps the rule to (event, tool
    atoms) pairs; an empty atom tuple means every call of the event. ``module``
    defaults to ``vsdd.rules.<id with dashes as underscores>``."""

    id: str
    on: tuple[tuple[str, tuple[str, ...]], ...]
    kind: str  # "gate" | "advisor" | "recorder"
    fail: str  # "closed" | "open"
    locked: bool = False
    spike_exempt: bool = False
    rewrites: bool = False  # may return Rewrite; step-results only
    summary: str = ""
    module: str = ""

    def load(self) -> Callable:
        """Import the rule module and return its ``check``. Called inside the
        rule's own ``try``, so an import crash is that rule's error."""
        name = self.module or ("vsdd.rules." + self.id.replace("-", "_"))
        module = importlib.import_module(name)
        return module.check


# --- the context ----------------------------------------------------------

_FILE_TOOL_FIELDS = (
    ("Read", "file_path"),
    ("Edit", "file_path"),
    ("Write", "file_path"),
    ("MultiEdit", "file_path"),
    ("NotebookEdit", "notebook_path"),
    ("Grep", "path"),
    ("Glob", "path"),
)


class Ctx:
    """Everything one call gives a rule ([[topic:rule-model]] 2.5).

    Direct fields are read straight from the payload (a missing string is
    ``""``); ``mode`` is the launch mode and ``root`` the session project root
    the launcher exports. Derived fields compute on first use.
    """

    def __init__(self, payload: dict, env: Mapping[str, str], mode: str, root: str | None):
        self.raw: dict = payload
        self.mode = mode
        self.root = root
        self.event: str = payload.get("hook_event_name", "") or ""
        self.tool: str = payload.get("tool_name", "") or ""
        tool_input = payload.get("tool_input")
        self.tool_input: dict = dict(tool_input) if isinstance(tool_input, dict) else {}
        self.cwd: str = payload.get("cwd", "") or ""
        self.permission_mode: str = payload.get("permission_mode", "") or ""
        #: the call's environment, as the launcher handed it over.
        self.env: dict = dict(env)
        #: the wall-clock start of the call; ``log`` reports ``ms`` against it.
        self.started = time.monotonic()
        #: one git subprocess per rule (hooks.md 2.4); the loop resets it.
        self._git_used = False
        self._cache: dict = {}

    # -- payload fields ----------------------------------------------------

    @property
    def session_id(self) -> str:
        return self.raw.get("session_id", "") or ""

    @property
    def agent_type(self) -> str:
        return self.raw.get("agent_type", "") or ""

    @property
    def agent_id(self) -> str:
        return self.raw.get("agent_id", "") or ""

    # -- derived, cached ----------------------------------------------------

    @property
    def target_literal(self) -> Path | None:
        """The absolute path the call names, symlinks unresolved. ``None``
        when the event-tool combination names no file field."""
        cached = self._cache.get("target_literal")
        if cached is _MISSING:
            return None
        if cached is not None:
            return cached
        field_name = None
        for tool, name in _FILE_TOOL_FIELDS:
            if self.tool == tool:
                field_name = name
                break
        value = self.tool_input.get(field_name) if field_name else None
        if not isinstance(value, str) or not value:
            if self.tool in ("Grep", "Glob") and self.cwd:
                # path or cwd: a Grep/Glob with no path names the call's cwd.
                path = Path(os.path.abspath(self.cwd))
                self._cache["target_literal"] = path
                return path
            self._cache["target_literal"] = _MISSING
            return None
        path = Path(value)
        if not path.is_absolute():
            base = self.cwd or os.getcwd()
            path = Path(base) / path
        path = Path(os.path.abspath(path))
        self._cache["target_literal"] = path
        return path

    @property
    def target(self) -> Path | None:
        """``target_literal`` with symlinks resolved."""
        literal = self.target_literal
        if literal is None:
            return None
        return Path(os.path.realpath(literal))

    @property
    def project(self):
        """The governing project: found from ``target``, else ``cwd``, else
        the session root; an outside path falls back to the session's project
        ([[topic:rule-model]]). A :class:`vsdd.project.Project` subclass, so a
        state function that takes a project takes it."""
        return self.project_at(self.target) if self.target is not None else self.project_at(None)

    def project_at(self, path) -> "_HookProject":
        """The governing project for ``path`` (``None`` falls back to ``cwd``,
        then the session root)."""
        key = ("project", str(path) if path is not None else None)
        cached = self._cache.get(key)
        if cached is not None:
            return cached
        roots = []
        if path is not None:
            roots.append(path)
        if self.cwd:
            roots.append(Path(self.cwd))
        if self.root:
            roots.append(Path(self.root))
        found = None
        focused = None
        for start in roots:
            found = _project.find_project_root(start, home=_home(self.env))
            if found is not None:
                focused = start
                break
        if found is None and self.root:
            # The launcher guarantees this is an initialised project; detect
            # for the worktree facts, never adopting a home-rooted state.
            found = Path(self.root)
        project = (
            _HookProject.build(found, self.env, classify_at=focused)
            if found is not None
            else None
        )
        self._cache[key] = project
        return project

    @property
    def command(self):
        """For ``Bash``: the parsed command ([[topic:rule-model]])."""
        if self.tool != "Bash":
            return None
        cached = self._cache.get("command")
        if cached is None:
            from . import shell

            cached = shell.parse(str(self.tool_input.get("command", "")), self.cwd or os.getcwd())
            self._cache["command"] = cached
        return cached

    @property
    def where(self) -> str:
        """The call's effective directory: the directory of ``target``, else
        ``cwd``. A ``Bash`` command has one ``where`` per segment."""
        if self.tool == "Bash":
            return self.cwd
        target = self.target
        if target is not None:
            return str(target if target.is_dir() else target.parent)
        return self.cwd

    @property
    def story(self) -> str | None:
        """The story whose branch is checked out in the worktree that contains
        ``where``, if any."""
        return self.story_at(self.where)

    def story_at(self, directory) -> str | None:
        """The story id for one directory (a segment's ``where``)."""
        if not directory:
            return None
        detected = _project.detect(directory)
        if detected is None or detected.worktree is None:
            return None
        if detected.worktree.kind == "story":
            return detected.worktree.id
        return None

    @property
    def in_spike(self) -> bool:
        """The caller's row is the spike row, or the worktree that contains
        ``where`` is a spike."""
        if self.row.spike:
            return True
        return self.spike_at(self.where)

    def spike_at(self, directory) -> bool:
        """Whether one directory (a segment's ``where``) sits in a spike."""
        if not directory:
            return False
        detected = _project.detect(directory)
        return detected is not None and detected.in_spike

    def branch_at(self, directory):
        """The checked-out branch of the worktree that holds ``directory``."""
        if not directory:
            return None
        from . import gitfs

        return gitfs.branch(directory)

    @property
    def row(self):
        """The wall-table row for ``agent_type`` (hooks.md 2.7)."""
        cached = self._cache.get("row")
        if cached is None:
            from . import policy

            try:
                cached = policy.tables().row(self.agent_type)
            except Exception as exc:  # a missing table is the reading rule's crash
                cached = _RowFault(exc)
                self._cache["row_fault"] = exc
            self._cache["row"] = cached
        fault = self._cache.get("row_fault")
        if fault is not None:
            raise fault
        return cached

    @property
    def classes(self) -> tuple[str, ...]:
        """The path classes that contain ``target`` (hook-runner-b 3.1);
        ``()`` when the call names no file. Class matching tests both forms
        (rule-model 2.5), so the literal goes to ``paths.classes_of``, which
        returns the union of the literal and the resolved form's classes. A
        policy table that does not load raises
        :class:`~vsdd.hooks.policy.PolicyInvalid`: the reading rule's
        crash."""
        literal = self.target_literal
        if literal is None:
            return ()
        return self.classes_of(literal)

    def classes_of(self, path, where=None) -> tuple[str, ...]:
        """The path classes of any path, the same function as
        ``paths.classes_of``; ``where`` defaults to the payload ``cwd`` for
        the computed ``own-worktree`` class."""
        from . import paths

        return paths.classes_of(path, self.project, where=where or self.cwd,
                                plugin_root=self.env.get("CLAUDE_PLUGIN_ROOT", ""))

    @property
    def new_content(self):
        """The content a Write/Edit/MultiEdit would leave — H4, not built."""
        raise ContextUnavailable(
            "ctx.new_content is built by H4 (hooks/edits.py); until then a rule "
            "that reads it is quarantinable"
        )

    # -- services ------------------------------------------------------------

    def log(self, type_: str, severity: str = "info", **fields) -> bool:
        """Add one event to the session project's log: ``agent_type``,
        ``agent_id``, ``event``, ``tool`` and ``ms`` added here. Write errors
        are swallowed — the decision is never changed by a log fault. A field
        the caller names wins over the automatic one."""
        automatic = {
            "agent_type": self.agent_type,
            "agent_id": self.agent_id,
            "event": self.event,
            "tool": self.tool,
            "ms": int((time.monotonic() - self.started) * 1000),
        }
        automatic.update(fields)
        try:
            log = self._event_log()
            if log is None:
                return False
            return log.log(type_, severity, **automatic)
        except Exception:
            return False  # the event log cannot take a call down with it

    def _event_log(self):
        if self.root is None:
            return None
        key = "event_log"
        cached = self._cache.get(key)
        if cached is None:
            override = self.env.get("VSDD_EVENTS_DIR")
            directory = Path(override) if override else _events.events_dir_for(self.root)
            session = self.session_id or "unknown"
            cached = _events.EventLog(directory, session)
            self._cache[key] = cached
        return cached

    def git_budget_reset(self) -> None:
        """Reset the one-git-subprocess budget; the loop does it per rule."""
        self._git_used = False

    def git_budget_mark_used(self) -> None:
        """Record that this rule has spent its one git subprocess."""
        self._git_used = True

    @property
    def git_budget_used(self) -> bool:
        return self._git_used


_MISSING = object()


class _RowFault:
    """A wall row that could not be loaded; re-raised on read."""

    def __init__(self, exc: Exception):
        self.exc = exc


def _home(env: Mapping[str, str]) -> str | None:
    home = env.get("HOME")
    return home or None


class _HookProject(_project.Project):
    """The governing project a rule sees: the state design's ``Project`` plus
    what rules read — the hooks directory, the project configuration with its
    fault record, and the policy tables (plan 3.2)."""

    @classmethod
    def build(cls, root, env: Mapping[str, str], classify_at=None) -> "_HookProject | None":
        base = _project.find_project_root(root, home=_home(env))
        if base is None:
            return None
        focus = classify_at if classify_at is not None else root
        worktree = _project.containing_worktree(focus, root=base)
        detected = _project.detect(focus)
        branch = detected.branch if detected is not None else None
        if detected is not None and detected.worktree is not None and worktree is None:
            # A story or spike recognised from its branch outside .worktrees.
            worktree = detected.worktree
        return cls(
            root=base,
            state_dir=base / _project.FACTORY_DIR,
            worktree=worktree,
            branch=branch,
            env=dict(env),
        )

    def __init__(self, root: Path, state_dir: Path, worktree, branch, env: dict):
        super().__init__(root=root, state_dir=state_dir, worktree=worktree, branch=branch)
        object.__setattr__(self, "_env", env)
        object.__setattr__(self, "_cache", {})
        object.__setattr__(self, "_config_loaded", False)

    # -- cached extras -------------------------------------------------------

    @property
    def hooks_dir(self) -> Path:
        """``<state>/local/hooks`` — the runner's own store: caller markers,
        spike admissions."""
        return self.state_dir / "local" / "hooks"

    @property
    def config(self) -> "_config.Config":
        """The project configuration; the defaults when it does not parse, so
        a broken file relaxes nothing and ``config_error`` says why."""
        self._load_config_once()
        return self._cache["config"]

    @property
    def config_error(self) -> Exception | None:
        """Why :attr:`config` fell back to the defaults, when it did."""
        self._load_config_once()
        return self._cache["config_error"]

    def _load_config_once(self) -> None:
        cache = self._cache
        if cache.get("_config_loaded"):
            return
        path = self.state_dir / _project.CONFIG_NAME
        error = None
        try:
            loaded = _config.load(path)
        except _config.ConfigError as exc:
            loaded = _config.loads("schema = 1\n")
            error = exc
        except OSError as exc:
            loaded = _config.loads("schema = 1\n")
            error = _config.ConfigError(f"cannot read {path}: {exc}")
        cache["config"] = loaded
        cache["config_error"] = error
        cache["_config_loaded"] = True

    @property
    def policy(self):
        """The shipped policy tables; raises :class:`policy.PolicyInvalid`
        when one is missing or invalid."""
        from . import policy

        return policy.tables()

    def rule_config(self, rule_id: str) -> RuleConfig:
        """The ``[hooks.rules.<id>]`` table of one rule: its strength and the
        remaining keys. A locked rule always gets ``block``. A rule whose own
        table is faulty resets to its defaults (SL1-47); any other key fault
        already reset the whole configuration in :mod:`vsdd.config`."""
        try:
            table = self.config.get("hooks", "rules")
        except (KeyError, TypeError):
            return RuleConfig()
        entry = table.get(rule_id)
        if not isinstance(entry, dict):
            return RuleConfig()
        options = {key: value for key, value in entry.items() if key != "strength"}
        strength = entry.get("strength", "block")
        if strength not in ("block", "advise", "off"):
            return RuleConfig(options=options)
        return RuleConfig(strength=strength, options=options)
