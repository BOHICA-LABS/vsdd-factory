"""Loads and validates the project configuration, ``.factory/factory.toml``
(state.md 2.12).

The file exists if and only if the project is initialised for 1.0. A person
edits it; code reads it. The example in state.md 2.12 is the whole key list —
every section and every key the loader accepts appears in it — and an unknown
key or a wrong type is an error, never a relaxation: an invalid value never
loosens a gate. That includes the free-form-looking sections: ``[[component]]``,
``[[reference]]``, ``[[repo]]`` and ``[discover]`` each accept exactly the keys
the example shows, and ``[caps]`` names come from the shipped artifact registry
(``py/vsdd/policy/artifacts.toml``), which also fixes the "at most twice its
default" bound.

Standard library only (``tomllib`` is 3.11+, the floor). Behaviour notes:

- defaults: unset keys take their design default, so a project file carries
  only the keys it cares about; arrays of tables replace the default empty
  list; ``caps.waived`` survives a ``[caps]`` that does not name it,
- ranges: counts, attempts, passes and minutes are at least 1; ratios
  (``holdout.*``, ``mutation.kill_rate``) lie in 0..1 — an out-of-range value
  never loosens a gate silently,
- floats: ``holdout.*`` and ``mutation.kill_rate`` accept an integer where a
  float is expected (TOML ``1`` for ``1.0``); ``schema`` must be the integer 1,
- ``[hooks]``: an unknown rule, key or bad value under ``[hooks.rules.*]`` does
  not disable anything and does not invalidate the whole file — the rule falls
  back to its built-in defaults and the problem is reported on
  :attr:`Config.hook_problems` for the runner to log as ``config.invalid``
  (hooks.md 2.6),
- ``[[repo]]`` entries may carry their own ``branches``, ``commands``,
  ``paths``, ``release`` and ``delivery`` tables, validated through the same
  code paths as the top-level sections (state.md 2.12: the same keys as the
  top level), and only in a ``layout = "hub"`` project,

The survey returns limits under the names the workflow prelude uses (blueprint
4.12); :meth:`Config.prelude_limits` is that mapping, and a test fails on an
unknown name.
"""

from __future__ import annotations

import tomllib
from pathlib import Path

__all__ = ["Config", "ConfigError", "load", "loads"]

SCHEMA_VERSION = 1

# leaf kinds
STR = "str"
INT = "int"
INT1 = "int1"  # an integer of at least 1: a count, attempt, pass or number of minutes
FLOAT = "float"
UNIT_RATIO = "unit_ratio"  # a ratio in 0..1
BOOL = "bool"
STR_LIST = "str_list"

_COMMAND_KEYS = (
    "build",
    "test",
    "test_report",
    "lint",
    "format_check",
    "run",
    "ui_check",
    "docs_check",
    "mutation",
    "scan",
    "audit",
    "bench",
    "proofs",
)

_PATH_CLASSES = ("tests", "harness", "ci", "repo_config", "manifests", "docs", "source")

_LOCKED_HOOK_RULES = (
    "read-walls",
    "write-scopes",
    "gate-guard",
    "decisions",
    "step-results",
    "session-briefing",
)

_HOOK_RULE_STRENGTHS = ("block", "advise", "off")

_AUTO_ALLOW_KEYS = ("scope",)
_AUTO_ALLOW_SCOPES = ("off", "core", "extended")

# The keys a rule takes of its own (config/hooks.md); strength belongs to
# every tunable rule and is validated apart. A rule id the runner does not
# know is the hook runner's check, not the loader's.
_HOOK_RULE_OWN_KEYS: dict[str, tuple] = {
    "bash-git": (("attribution", BOOL),),
    "bash-destructive": (("allow_pr_close", BOOL),),
    "secrets": (("secret_globs", STR_LIST),),
    "purity-advice": (("pure_paths", STR_LIST), ("side_effect_patterns", STR_LIST)),
}


def _refresh_protected_paths(data: dict) -> None:
    """The protected-paths default follows ``[release]`` (finding AU-T2-15):
    ``.github/**``, ``.env*``, ``CODEOWNERS``, then the files ``[release]``
    names — the change log and each version file."""
    release = data["release"]
    seen: list[str] = []
    for path in (
        ".github/**",
        ".env*",
        "CODEOWNERS",
        release["changelog"],
        *[entry["path"] for entry in release["version_files"]],
    ):
        if path not in seen:
            seen.append(path)
    data["delivery"]["protected_paths"] = seen


def _sec(**keys) -> dict:
    return keys


# section -> key -> (default, kind) or (default, kind, (choices,))
SPECS: dict[str, dict[str, tuple]] = {
    "project": _sec(
        name=("", STR),
        languages=([], STR_LIST),
        ui=(False, BOOL),
        layout=("single", STR, ("single", "hub")),
    ),
    "branches": _sec(
        integration=("develop", STR),
        release=("main", STR),
        protected=(["main"], STR_LIST),
        story=("story/{id}", STR),
        spike=("spike/{name}", STR),
    ),
    "commands": _sec(**{key: ("", STR) for key in _COMMAND_KEYS}),
    "paths": _sec(**{key: ([], STR_LIST) for key in _PATH_CLASSES}),
    "delivery": _sec(
        mode=("pr", STR, ("pr", "local")),
        ci=("required", STR, ("required", "none")),
        merge=("auto", STR, ("auto", "human")),
        merge_strategy=("squash", STR),
        protected_paths=(
            [".github/**", ".env*", "CODEOWNERS", "CHANGELOG.md"],
            STR_LIST,
        ),
        dependency_changes=("human", STR, ("human", "auto")),
        ci_wait_minutes=(30, INT1),
    ),
    "convergence": _sec(passes=(3, INT1), rounds=(3, INT1)),
    "limits": _sec(
        agents_per_run=(900, INT1),
        agents_per_story=(40, INT1),
        stories_per_wave_run=(24, INT1),
        step_attempts=(2, INT1),
        delivery_rounds=(3, INT1),
        gate_minutes=(60, INT1),
        parallel_gates=(4, INT1),
        failed_runs_before_stuck=(2, INT1),
    ),
    "spikes": _sec(
        per_step=(2, INT1),
        per_run=(6, INT1),
        per_day=(20, INT1),
        minutes=(20, INT1),
        max_open=(3, INT1),
        retain_days=(14, INT1),
    ),
    "holdout": _sec(
        mean_min=(0.85, UNIT_RATIO),
        must_pass_min=(0.6, UNIT_RATIO),
        stddev_max=(0.15, UNIT_RATIO),
        rotation=(0.8, UNIT_RATIO),
        batch=(12, INT1),
    ),
    "mutation": _sec(
        kill_rate=({"critical": 0.95, "high": 0.90, "medium": 0.80, "low": 0.70}, "crit_floats"),
    ),
    "stops": _sec(
        plan=("auto", STR, ("auto", "always")),
        plan_ceiling_agents=(600, INT1),
    ),
    "evidence": _sec(wave_demo=(False, BOOL), final_demo=(False, BOOL)),
    "maintain": _sec(sweeps=([], STR_LIST), fixes_per_run=(10, INT1)),
    "release": _sec(
        version_files=([], "version_files"),
        changelog=("CHANGELOG.md", STR),
        tag_format=("v{version}", STR),
        publish=([], STR_LIST),
        github_release=(True, BOOL),
    ),
    "state": _sec(
        push=(True, BOOL),
        lock_stale_minutes=(20, INT1),
        retain_runs_days=(30, INT1),
        prune_closed_cycles=(True, BOOL),
    ),
    "notify": _sec(
        channel=("none", STR, ("none", "github-issue", "command")),
        command=("", STR),
    ),
    "hooks": _sec(
        main_session_wall=(True, BOOL),
        log_retention_days=(30, INT1),
        rules=({}, "hook_rules"),
    ),
}

_FREEFORM_SECTIONS = ("caps", "component", "reference", "repo", "discover")

_KILL_RATE_CRITICALITIES = ("critical", "high", "medium", "low")


def _defaults() -> dict:
    data: dict = {"schema": SCHEMA_VERSION}
    for section, keys in SPECS.items():
        data[section] = {key: spec[0] for key, spec in keys.items()}
    data["commands"]["env"] = {}
    data["discover"] = {"source": []}
    data["caps"] = {"waived": []}
    data["component"] = []
    data["reference"] = []
    data["repo"] = []
    return data


class ConfigError(ValueError):
    """The configuration file is outside the key table; the briefing reports it."""


def _type(value: object) -> str:
    if isinstance(value, bool):
        return "boolean"
    if isinstance(value, int):
        return "integer"
    if isinstance(value, float):
        return "float"
    if isinstance(value, str):
        return "string"
    if isinstance(value, list):
        return "list"
    if isinstance(value, dict):
        return "table"
    return type(value).__name__


def _check_scalar(value: object, kind: str, where: str) -> object:
    if kind == STR:
        if not isinstance(value, str):
            raise ConfigError(f"{where} must be a string, got {_type(value)}")
        return value
    if kind == INT:
        if isinstance(value, bool) or not isinstance(value, int):
            raise ConfigError(f"{where} must be an integer, got {_type(value)}")
        return value
    if kind == INT1:
        if isinstance(value, bool) or not isinstance(value, int):
            raise ConfigError(f"{where} must be an integer, got {_type(value)}")
        if value < 1:
            raise ConfigError(f"{where} must be at least 1, got {value}")
        return value
    if kind == FLOAT:
        if isinstance(value, bool) or not isinstance(value, (int, float)):
            raise ConfigError(f"{where} must be a number, got {_type(value)}")
        return float(value)
    if kind == UNIT_RATIO:
        if isinstance(value, bool) or not isinstance(value, (int, float)):
            raise ConfigError(f"{where} must be a number, got {_type(value)}")
        number = float(value)
        if not 0.0 <= number <= 1.0:
            raise ConfigError(f"{where} must lie between 0 and 1, got {value}")
        return number
    if kind == BOOL:
        if not isinstance(value, bool):
            raise ConfigError(f"{where} must be a boolean, got {_type(value)}")
        return value
    if kind == STR_LIST:
        if not isinstance(value, list) or any(
            isinstance(item, bool) or not isinstance(item, str) for item in value
        ):
            raise ConfigError(f"{where} must be a list of strings, got {_type(value)}")
        return list(value)
    raise ConfigError(f"{where}: unknown kind {kind!r}")


def _apply_leaf(target: dict, spec: tuple, value: object, where: str) -> None:
    key = where.rsplit(".", 1)[-1]
    if len(spec) >= 3 and isinstance(spec[2], tuple):
        checked = _check_scalar(value, spec[1], where)
        if checked not in spec[2]:
            raise ConfigError(
                f"{where} must be one of {'|'.join(spec[2])}, got {checked!r}"
            )
        target[key] = checked
    else:
        target[key] = _check_scalar(value, spec[1], where)


def _composite_handlers(target: dict, section: str, problems: dict | None) -> dict:
    """The keys of ``section`` whose values are not plain leaves, with the
    container they write into (``target``), so a per-repo override and the
    top-level section validate through the same code."""
    handlers: dict[str, callable] = {}
    if section == "commands":

        def env_handler(_data, value, where):
            target["env"] = _str_table(value, where)

        handlers["env"] = env_handler
    if section == "hooks":
        handlers["rules"] = lambda _data, value, where: _apply_hook_rules(
            target, value, where, problems
        )
    if section == "release":
        handlers["version_files"] = lambda _data, value, where: target.__setitem__(
            "version_files", _version_files(value, where)
        )
    if section == "mutation":
        handlers["kill_rate"] = lambda _data, value, where: target.__setitem__(
            "kill_rate", _kill_rates(value, where)
        )
    return handlers


def _apply_table(
    data: dict, section: str, incoming: dict, where: str, target: dict | None = None
) -> None:
    """Fold one section table in, honouring the section's composite keys.

    ``target`` redirects the writes into a per-repo dict, so an override table
    takes exactly the route the top-level section takes.
    """
    specs = SPECS[section]
    if target is None:
        target = data[section]
    problems = data.get("_hook_problems") if section == "hooks" else None
    known_extra = _composite_handlers(target, section, problems)

    for key, value in incoming.items():
        if key in known_extra:
            known_extra[key](data, value, f"{where}.{key}")
            continue
        if key not in specs:
            raise ConfigError(
                f"{where}.{key}: unknown key (known: {', '.join(sorted(specs))})"
            )
        _apply_leaf(target, specs[key], value, f"{where}.{key}")


def _str_table(incoming: object, where: str) -> dict:
    if not isinstance(incoming, dict):
        raise ConfigError(f"{where} must be a table, got {_type(incoming)}")
    table: dict = {}
    for key, value in incoming.items():
        if not isinstance(value, str):
            raise ConfigError(f"{where}.{key} must be a string, got {_type(value)}")
        table[key] = value
    return table


def _kill_rates(incoming: object, where: str) -> dict:
    if not isinstance(incoming, dict):
        raise ConfigError(f"{where} must be a table, got {_type(incoming)}")
    rates: dict = {}
    for criticality, value in incoming.items():
        if criticality not in _KILL_RATE_CRITICALITIES:
            raise ConfigError(
                f"{where}.{criticality}: unknown criticality "
                f"(known: {', '.join(_KILL_RATE_CRITICALITIES)})"
            )
        checked = _check_scalar(value, UNIT_RATIO, f"{where}.{criticality}")
        rates[criticality] = checked
    for criticality in _KILL_RATE_CRITICALITIES:
        rates.setdefault(criticality, SPECS["mutation"]["kill_rate"][0][criticality])
    return rates


def _apply_hook_rules(container: dict, incoming: object, where: str, problems: dict | None) -> None:
    """Validate ``[hooks.rules.*]`` per rule, never for the whole file.

    The keys under a rule are closed (config/hooks.md; finding P3SEAM-06):
    ``strength``, the keys the rule takes of its own, and for ``auto-allow``
    only ``scope``. A fault under a rule costs that rule its settings only —
    it falls back to its built-in defaults, reported in ``problems`` keyed by
    rule id — while a fault in ``[hooks]`` itself is a ``ConfigError``
    (finding SL1-47, decision 6). Whether a rule id exists is the hook
    runner's check.
    """
    if not isinstance(incoming, dict):
        raise ConfigError(f"{where} must be a table, got {_type(incoming)}")
    collected = {} if problems is None else problems
    rules: dict = {}
    for name, table in incoming.items():
        problems_of_rule: list[str] = []
        if not isinstance(table, dict):
            collected[name] = [f"{where}.{name} must be a table, got {_type(table)}"]
            continue
        rules_entry: dict = {}
        if name == "auto-allow":
            scope = table.get("scope", "off")
            if scope not in _AUTO_ALLOW_SCOPES:
                problems_of_rule.append(
                    f"{where}.{name}.scope must be one of "
                    f"{'|'.join(_AUTO_ALLOW_SCOPES)}, got {scope!r}"
                )
            for key in table:
                if key not in _AUTO_ALLOW_KEYS:
                    problems_of_rule.append(
                        f"{where}.{name}.{key}: unknown key (known: {', '.join(_AUTO_ALLOW_KEYS)})"
                    )
            if not problems_of_rule:
                rules_entry = {"scope": scope}
        else:
            own_keys = dict(_HOOK_RULE_OWN_KEYS.get(name, ()))
            strength = table.get("strength")
            if "strength" in table:
                if strength not in _HOOK_RULE_STRENGTHS:
                    problems_of_rule.append(
                        f"{where}.{name}.strength must be one of "
                        f"{'|'.join(_HOOK_RULE_STRENGTHS)}, got {strength!r}"
                    )
                if name in _LOCKED_HOOK_RULES:
                    problems_of_rule.append(
                        f"{where}.{name}: {name} is locked; setting strength is a "
                        "configuration error (hooks.md 2.6)"
                    )
            for key in table:
                if key != "strength" and key not in own_keys:
                    problems_of_rule.append(
                        f"{where}.{name}.{key}: unknown key (known: "
                        f"{', '.join(sorted({'strength', *own_keys}))})"
                    )
            if not problems_of_rule:
                if "strength" in table and name not in _LOCKED_HOOK_RULES and strength != "block":
                    rules_entry["strength"] = strength
                for key, kind in own_keys.items():
                    if key in table:
                        try:
                            rules_entry[key] = _check_scalar(
                                table[key], kind, f"{where}.{name}.{key}"
                            )
                        except ConfigError as exc:
                            problems_of_rule.append(str(exc))
        if problems_of_rule:
            collected[name] = problems_of_rule
        elif rules_entry:
            rules[name] = rules_entry
    container["rules"] = rules


def _version_files(incoming: object, where: str) -> list:
    if not isinstance(incoming, list):
        raise ConfigError(f"{where} must be a list of tables, got {_type(incoming)}")
    entries: list[dict] = []
    for index, entry in enumerate(incoming):
        item_where = f"{where}[{index}]"
        if not isinstance(entry, dict):
            raise ConfigError(f"{item_where} must be a table, got {_type(entry)}")
        if not isinstance(entry.get("path"), str) or not entry["path"]:
            raise ConfigError(f"{item_where}.path must be a non-empty string")
        cleaned: dict = {"path": entry["path"]}
        if "key" in entry:
            if not isinstance(entry["key"], str):
                raise ConfigError(f"{item_where}.key must be a string")
            cleaned["key"] = entry["key"]
        entries.append(cleaned)
    return entries


def _apply_hooks(data: dict, incoming: object, where: str) -> None:
    _apply_table(data, "hooks", incoming, where)


def _artifact_caps() -> dict[str, int]:
    """The size caps the artifact registry ships, as ``kind -> default bytes``.

    The registry (``py/vsdd/policy/artifacts.toml``) is the single definition of
    the cap names and defaults (state.md 2.12; the C2 policy tables). Loaded on
    first use and cached: the registry ships with the plugin and never changes
    while it runs.
    """
    global _ARTIFACT_CAPS
    if _ARTIFACT_CAPS is None:
        from .policy import load

        registry = load("artifacts")
        _ARTIFACT_CAPS = {
            kind: row["max_bytes"]
            for kind, row in registry.items()
            if isinstance(row, dict) and isinstance(row.get("max_bytes"), int)
        }
    return _ARTIFACT_CAPS


_ARTIFACT_CAPS: dict[str, int] | None = None


def _apply_caps(data: dict, incoming: object, where: str) -> None:
    if not isinstance(incoming, dict):
        raise ConfigError(f"{where} must be a table, got {_type(incoming)}")
    known = _artifact_caps()
    caps: dict = {"waived": list(data["caps"]["waived"])}
    for key, value in incoming.items():
        if key == "waived":
            caps["waived"] = _check_scalar(value, STR_LIST, f"{where}.waived")
            continue
        if key not in known:
            raise ConfigError(
                f"{where}.{key}: unknown cap; a cap names an artifact kind with a "
                f"size limit ({', '.join(sorted(known))})"
            )
        number = _check_scalar(value, INT, f"{where}.{key}")
        default = known[key]
        # A raised cap is at most twice its default and never below one byte
        # (state.md 2.12); an out-of-range value never loosens a gate silently.
        if not 1 <= number <= 2 * default:
            raise ConfigError(
                f"{where}.{key} must lie between 1 and twice its default "
                f"({2 * default}); the {key} cap defaults to {default}"
            )
        caps[key] = number
    data["caps"] = caps


def _apply_discover(data: dict, incoming: object, where: str) -> None:
    if not isinstance(incoming, dict):
        raise ConfigError(f"{where} must be a table, got {_type(incoming)}")
    known = ("source",)
    for key in incoming:
        if key not in known:
            raise ConfigError(
                f"{where}.{key}: unknown key (known: {', '.join(known)})"
            )
    source = incoming.get("source", [])
    if not isinstance(source, list):
        raise ConfigError(f"{where}.source must be a list of tables, got {_type(source)}")
    entries: list[dict] = []
    for index, entry in enumerate(source):
        item_where = f"{where}.source[{index}]"
        if not isinstance(entry, dict):
            raise ConfigError(f"{item_where} must be a table, got {_type(entry)}")
        for required in ("name", "command"):
            if not isinstance(entry.get(required), str) or not entry[required]:
                raise ConfigError(f"{item_where}.{required} must be a non-empty string")
        for key in entry:
            if key not in ("name", "command"):
                raise ConfigError(
                    f"{item_where}.{key}: unknown key (known: name, command)"
                )
        entries.append({"name": entry["name"], "command": entry["command"]})
    data["discover"]["source"] = entries


def _apply_component(data: dict, incoming: object, where: str) -> None:
    """``[[component]]``: a sub-tree with toolchain-specific commands.

    Closed keys: ``name`` (non-empty string), ``paths`` (list of strings),
    ``commands`` (a table whose keys are ``[commands]`` keys). ``[[component]]``
    is the section most able to weaken the green gate (hooks.md 2.6), so a
    misspelt command key is an error, never a silent fall-through.
    """
    if not isinstance(incoming, list):
        raise ConfigError(f"{where} must be a list of tables, got {_type(incoming)}")
    entries: list[dict] = []
    for index, entry in enumerate(incoming):
        item_where = f"{where}[{index}]"
        if not isinstance(entry, dict):
            raise ConfigError(f"{item_where} must be a table, got {_type(entry)}")
        cleaned: dict = {}
        for key, value in entry.items():
            if key == "name":
                cleaned["name"] = _check_scalar(value, STR, f"{item_where}.name")
                if not cleaned["name"]:
                    raise ConfigError(f"{item_where}.name must be a non-empty string")
            elif key == "paths":
                cleaned["paths"] = _check_scalar(value, STR_LIST, f"{item_where}.paths")
            elif key == "commands":
                if not isinstance(value, dict):
                    raise ConfigError(
                        f"{item_where}.commands must be a table, got {_type(value)}"
                    )
                commands: dict = {}
                for command_key, command_value in value.items():
                    if command_key not in _COMMAND_KEYS:
                        raise ConfigError(
                            f"{item_where}.commands.{command_key}: unknown key "
                            f"(known: {', '.join(_COMMAND_KEYS)})"
                        )
                    if not isinstance(command_value, str):
                        raise ConfigError(
                            f"{item_where}.commands.{command_key} must be a string, "
                            f"got {_type(command_value)}"
                        )
                    commands[command_key] = command_value
                cleaned["commands"] = commands
            else:
                raise ConfigError(
                    f"{item_where}.{key}: unknown key (known: commands, name, paths)"
                )
        if not isinstance(cleaned.get("name"), str) or not cleaned.get("name"):
            raise ConfigError(f"{item_where}.name must be a non-empty string")
        entries.append(cleaned)
    data["component"] = entries


def _apply_reference(data: dict, incoming: object, where: str) -> None:
    """``[[reference]]``: a repository studied during ingestion. Closed keys: ``name``, ``url``."""
    if not isinstance(incoming, list):
        raise ConfigError(f"{where} must be a list of tables, got {_type(incoming)}")
    entries: list[dict] = []
    for index, entry in enumerate(incoming):
        item_where = f"{where}[{index}]"
        if not isinstance(entry, dict):
            raise ConfigError(f"{item_where} must be a table, got {_type(entry)}")
        for key in entry:
            if key not in ("name", "url"):
                raise ConfigError(
                    f"{item_where}.{key}: unknown key (known: name, url)"
                )
        cleaned: dict = {}
        for key in ("name", "url"):
            if not isinstance(entry.get(key), str) or not entry[key]:
                raise ConfigError(f"{item_where}.{key} must be a non-empty string")
            cleaned[key] = entry[key]
        entries.append(cleaned)
    data["reference"] = entries


_REPO_OVERRIDES = ("branches", "commands", "paths", "release", "delivery")


def _apply_repo(data: dict, incoming: object, *, layout: str) -> None:
    """``[[repo]]``: one member repository of a hub.

    Closed keys: ``name``, ``path``, ``url`` and the five override tables. An
    override validates through the same code path as the top level (the same
    keys, the same composites — ``[repo.commands.env]``,
    ``[repo.release.version_files]``, ``[repo.delivery.protected_paths]`` —
    and the same ranges), so a hub project's per-repo release settings are a
    valid configuration, not one that silently drops every section to defaults.
    """
    if layout != "hub":
        raise ConfigError(
            "[[repo]] entries need project.layout = \"hub\"; this project is "
            f"{layout!r}"
        )
    if not isinstance(incoming, list):
        raise ConfigError(f"[[repo]] must be a list of tables, got {_type(incoming)}")
    repos: list[dict] = []
    seen: set[str] = set()
    for index, entry in enumerate(incoming):
        item_where = f"[[repo]][{index}]"
        if not isinstance(entry, dict):
            raise ConfigError(f"{item_where} must be a table, got {_type(entry)}")
        for required in ("name", "path", "url"):
            if not isinstance(entry.get(required), str) or not entry[required]:
                raise ConfigError(f"{item_where}.{required} must be a non-empty string")
        if entry["name"] in seen:
            raise ConfigError(f"{item_where}: duplicate repo name {entry['name']!r}")
        seen.add(entry["name"])
        cleaned: dict = {"name": entry["name"], "path": entry["path"], "url": entry["url"]}
        for key in entry:
            if key not in ("name", "path", "url") and key not in _REPO_OVERRIDES:
                raise ConfigError(
                    f"{item_where}.{key}: unknown key "
                    f"(known: {', '.join(('name', 'path', 'url') + _REPO_OVERRIDES)})"
                )
        for override in _REPO_OVERRIDES:
            if override not in entry:
                continue
            sub = entry[override]
            if not isinstance(sub, dict):
                raise ConfigError(f"{item_where}.{override} must be a table")
            # Seed the override with the section defaults, then fold the
            # incoming keys in through the same _apply_table route as the top
            # level: composites (env, version_files, protected_paths,
            # kill_rate) and ranges behave identically.
            target = {key: spec[0] for key, spec in SPECS[override].items()}
            if override == "commands":
                target["env"] = {}
            _apply_table(data, override, sub, f"{item_where}.{override}", target=target)
            cleaned[override] = target
        repos.append(cleaned)
    data["repo"] = repos


class Config:
    """A validated project configuration; ``data`` is the merged dictionary."""

    def __init__(self, data: dict, path: Path | None = None):
        self.data = data
        self.path = path
        #: problems gathered per ``[hooks.rules.*]`` rule: the rule fell back to
        #: its defaults and the runner logs ``config.invalid`` (hooks.md 2.6).
        self.hook_problems: dict[str, list[str]] = data.pop("_hook_problems", {})

    def get(self, section: str, key: str | None = None):
        if key is None:
            return self.data[section]
        return self.data[section][key]

    def hook_rules(self) -> dict[str, dict]:
        """The validated ``[hooks.rules.*]`` tables, one entry per rule that
        left its defaults. A rule id the runner does not know is reported by
        the runner, not here."""
        return dict(self.data["hooks"]["rules"])

    def command(self, name: str) -> str:
        """One of ``[commands]``, defaulting to empty: a missing command is a state."""
        return str(self.data["commands"].get(name, ""))

    def prelude_limits(self) -> dict:
        """Limits under the names the workflow prelude uses (blueprint 4.12)."""
        limits = self.data["limits"]
        return {
            "agents": limits["agents_per_run"],
            "story_agents": limits["agents_per_story"],
            "stories_per_run": limits["stories_per_wave_run"],
            "attempts": limits["step_attempts"],
            "delivery_rounds": limits["delivery_rounds"],
            "gate_minutes": limits["gate_minutes"],
            "ci_minutes": self.data["delivery"]["ci_wait_minutes"],
            "passes": self.data["convergence"]["passes"],
            "rounds": self.data["convergence"]["rounds"],
            "spikes_per_step": self.data["spikes"]["per_step"],
            "spikes_per_run": self.data["spikes"]["per_run"],
            "spike_minutes": self.data["spikes"]["minutes"],
        }

    def __repr__(self) -> str:  # pragma: no cover - debugging aid
        return f"<Config {self.path or ''}>"


def loads(text: str, path: Path | None = None) -> Config:
    """Parse and validate the configuration ``text``."""
    try:
        raw = tomllib.loads(text)
    except tomllib.TOMLDecodeError as exc:
        raise ConfigError(f"{path or 'factory.toml'}: invalid TOML: {exc}") from exc

    data = _defaults()

    schema = raw.get("schema")
    if schema is None:
        raise ConfigError("the schema key is missing (must be 1)")
    if isinstance(schema, bool) or isinstance(schema, float) or schema != SCHEMA_VERSION:
        raise ConfigError(f"schema must be the integer {SCHEMA_VERSION}, got {schema!r}")

    layout = (raw.get("project") or {}).get("layout", "single")
    protected_given = isinstance(raw.get("delivery"), dict) and "protected_paths" in raw["delivery"]
    for section, incoming in raw.items():
        if section == "schema":
            continue
        known = section in SPECS or section in _FREEFORM_SECTIONS
        if not known:
            raise ConfigError(
                f"unknown section [{section}] (known: "
                f"{', '.join(sorted(list(SPECS) + list(_FREEFORM_SECTIONS)))})"
            )
        if section in SPECS:
            if not isinstance(incoming, dict):
                raise ConfigError(f"[{section}] must be a table, got {_type(incoming)}")
            if section == "hooks":
                data.setdefault("_hook_problems", {})
            _apply_table(data, section, incoming, f"[{section}]")
        elif section == "caps":
            _apply_caps(data, incoming, "[caps]")
        elif section == "discover":
            _apply_discover(data, incoming, "[discover]")
        elif section == "component":
            _apply_component(data, incoming, "[[component]]")
        elif section == "reference":
            _apply_reference(data, incoming, "[[reference]]")
        elif section == "repo":
            _apply_repo(data, incoming, layout=layout)
    if not protected_given:
        _refresh_protected_paths(data)
    return Config(data, path)


def load(path: str | Path) -> Config:
    """Read, parse and validate one ``factory.toml``."""
    path = Path(path)
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ConfigError(f"cannot read {path}: {exc}") from exc
    return loads(text, path)
