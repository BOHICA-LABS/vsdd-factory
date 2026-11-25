"""The loader's key list is the one list (findings AU-S2-03, AU-T2-13,
AU-T2-15, P3SEAM-06, SL1-47, SL1-49, AU-P2-03): the five tests of the plan
plus the config command."""

from __future__ import annotations

import re
import tomllib
from pathlib import Path

import pytest

from vsdd.config import ConfigError, loads, SPECS

from ._repo import make_project


def test_key_table_parity():
    """config.SPECS is the one key list (decision 5): the shipped sample is
    an instance of it. Every key it carries is in SPECS, every section it
    carries is a SPECS section, and it loads clean."""
    sample = (
        Path(__file__).resolve().parents[2]
        / "tests" / "e2e" / "sample-project" / ".factory" / "factory.toml"
    )
    text = sample.read_text(encoding="utf-8")
    raw = tomllib.loads(text)
    loads(text)  # an unknown key or wrong type would raise here
    for section, incoming in raw.items():
        if section in ("schema", "caps", "discover"):
            continue
        assert section in SPECS, section
        for key, value in incoming.items():
            if isinstance(value, dict):  # a composite like [commands.env]
                continue
            assert key in SPECS[section], f"{section}.{key} is not in SPECS"
    # Loading fills every SPECS key, whichever of them the file carries: the
    # loader's defaults are the strict ones and nothing is left undefined.
    cfg = loads(text)
    for section, keys in SPECS.items():
        for key in keys:
            if section == "hooks" and key == "rules":
                continue
            assert key in cfg.data[section], f"{section}.{key} is unfilled"


def test_prelude_limits_equal_prelude_keys():
    """The limits the survey returns come from SPECS alone (AU-P2-03,
    AU-S2-09): names in prelude_limits, defaults from SPECS."""
    cfg = loads("schema = 1\n")
    limits = cfg.prelude_limits()
    assert limits["agents"] == SPECS["limits"]["agents_per_run"][0]
    assert limits["story_agents"] == SPECS["limits"]["agents_per_story"][0]
    assert limits["stories_per_run"] == SPECS["limits"]["stories_per_wave_run"][0]
    assert limits["attempts"] == SPECS["limits"]["step_attempts"][0]
    assert limits["delivery_rounds"] == SPECS["limits"]["delivery_rounds"][0]
    assert limits["gate_minutes"] == SPECS["limits"]["gate_minutes"][0]
    assert limits["ci_minutes"] == SPECS["delivery"]["ci_wait_minutes"][0]
    assert limits["passes"] == SPECS["convergence"]["passes"][0]
    assert limits["rounds"] == SPECS["convergence"]["rounds"][0]
    assert limits["spikes_per_step"] == SPECS["spikes"]["per_step"][0]
    assert limits["spikes_per_run"] == SPECS["spikes"]["per_run"][0]
    assert limits["spike_minutes"] == SPECS["spikes"]["minutes"][0]


def test_protected_paths_default_follows_release():
    cfg = loads("schema = 1\n")
    base = cfg.get("delivery", "protected_paths")
    assert base == [".github/**", ".env*", "CODEOWNERS", "CHANGELOG.md"]
    raised = loads(
        "schema = 1\n"
        '[release]\nchangelog = "RELEASES.md"\n'
        '[[release.version_files]]\npath = "src/version.py"\n'
    )
    assert raised.get("delivery", "protected_paths") == [
        ".github/**",
        ".env*",
        "CODEOWNERS",
        "RELEASES.md",
        "src/version.py",
    ]
    # A file that sets protected_paths is taken as written.
    explicit = loads(
        "schema = 1\n[delivery]\nprotected_paths = [\"custom/**\"]\n"
    )
    assert explicit.get("delivery", "protected_paths") == ["custom/**"]


def test_hook_rule_keys_closed_per_rule():
    cfg = loads(
        "schema = 1\n"
        "[hooks.rules.bash-git]\nattribution = true\n"
        "[hooks.rules.secrets]\nsecret_globs = [\"*.pem\"]\n"
        "[hooks.rules.read-walls]\nstrength = \"advise\"\n"
    )
    rules = cfg.hook_rules()
    assert rules["bash-git"] == {"attribution": True}
    assert rules["secrets"] == {"secret_globs": ["*.pem"]}
    # A locked rule ignores strength: the setting is reported, not applied.
    assert "read-walls" in cfg.hook_problems
    # An unknown key under a rule costs that rule its settings only.
    faulty = loads(
        "schema = 1\n"
        "[hooks.rules.bash-git]\nattribution = true\nwrong_key = 1\n"
        "[hooks.rules.purity-advice]\npure_paths = [\"src/**\"]\n"
    )
    assert "bash-git" in faulty.hook_problems
    assert "purity-advice" in faulty.hook_rules()
    assert faulty.hook_rules()["purity-advice"] == {"pure_paths": ["src/**"]}


def test_hook_fault_scope():
    # A fault under [hooks] itself is a ConfigError; a fault under a rule is
    # that rule's problem only (SL1-47, decision 6).
    with pytest.raises(ConfigError):
        loads('schema = 1\n[hooks]\nmain_session_wall = "yes-please"\n')
    with pytest.raises(ConfigError):
        loads("schema = 1\n[hooks]\nunknow_key = 1\n")
    # A non-table rule costs that rule its settings, not the file.
    soft = loads('schema = 1\n[hooks.rules]\nsecrets = "not-a-table"\n')
    assert set(soft.hook_problems) == {"secrets"}
    surviving = loads(
        "schema = 1\n"
        "[hooks.rules.bash-git]\nstrength = \"advise\"\n"
        "[hooks.rules.secrets]\nsecret_globs = \"oops\"\n"
    )
    assert surviving.hook_rules()["bash-git"] == {"strength": "advise"}
    assert set(surviving.hook_problems) == {"secrets"}


def test_raised_cap_applies_and_waived_skips():
    from vsdd.state import artifacts

    registry = artifacts.registry()
    kind = registry.kinds["story"]
    # 19 lines of 900 characters: over the 16,000-byte story cap, but every
    # body line inside the 1,000-character line rule.
    base = "\n".join("x" * 900 for _ in range(19))
    text = (
        "---\nid: S-9.01\ntitle: Big\nstatus: draft\ncycle: C-1\nkind: feature\n"
        "priority: P1\ncriticality: high\ncontracts: [BC-2.03.007]\n---\n\n"
        "## Acceptance criteria\n\n### AC-1 (BC-2.03.007)\n\n" + base + "\n"
    )
    config = loads("schema = 1\n")
    problems = [
        p.code
        for p in artifacts.check_text(
            ".factory/stories/E-9/S-9.01.md", text, config=config
        )
    ]
    assert "size_cap" in problems
    # The raised cap is the validated value: at most twice the default.
    raised = loads("schema = 1\n[caps]\nstory = 32000\n")
    problems = [
        p.code
        for p in artifacts.check_text(
            ".factory/stories/E-9/S-9.01.md", text, config=raised
        )
    ]
    assert "size_cap" not in problems
    # A waived file skips the cap check altogether.
    waived = loads(
        'schema = 1\n[caps]\nwaived = [".factory/stories/E-9/S-9.01.md"]\n'
    )
    problems = [
        p.code
        for p in artifacts.check_text(
            ".factory/stories/E-9/S-9.01.md", text, config=waived
        )
    ]
    assert "size_cap" not in problems


def test_config_command_prints_merged_data(tmp_path):
    from vsdd.cli import dispatch
    import io
    import json

    root = make_project(tmp_path / "cfg")
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(
        ["-C", str(root), "config", "--json"],
        stdout=out,
        stderr=err,
        stdin_tty=True,
    )
    assert code == 0, err.getvalue()
    payload = json.loads(out.getvalue())
    assert payload["ok"] is True
    assert payload["config"]["schema"] == 1
    assert payload["hook_problems"] == {}
    assert payload["config"]["project"]["name"] == root.name


def _written_sections(root) -> dict:
    """The sections and keys ``init`` actually writes, parsed back."""
    raw = tomllib.loads((root / ".factory" / "factory.toml").read_text(encoding="utf-8"))
    raw.pop("schema", None)
    if "hooks" in raw:
        raw["hooks"].pop("rules", None)  # the --auto-allow scope, a composite
    return raw


def test_init_writes_exactly_the_specs_keys(tmp_path):
    """The rendered factory.toml is an instance of SPECS (decision 5): init
    states no key or default a second time (findings S3R-state-core-1,
    AU-S2-03, AU-T2-13). Every fixed section carries exactly the keys of its
    SPECS row, so a key dropped from the renderer, an added hand-typed one or
    a written protected_paths all fail here."""
    root = make_project(tmp_path / "parity")
    written = _written_sections(root)
    written_sections = set(written)
    expected = set(SPECS)  # [project] closes the file, every section present
    assert written_sections == expected, sorted(written_sections ^ expected)
    # delivery.protected_paths is the loader's computation from [release];
    # a written copy would freeze the AU-T2-15 fix at init time.
    assert "protected_paths" not in written["delivery"]
    # hooks.rules is a composite the loader fills; init writes only leaves.
    assert set(written["hooks"]) == {"main_session_wall", "log_retention_days"}
    # Every fixed section's written keys equal its SPECS row minus the two
    # composites: a section key init forgets to write shows here.
    for section, keys in SPECS.items():
        expected_keys = set(keys) - {"protected_paths", "rules"}
        assert set(written[section]) == expected_keys, (
            section,
            sorted(set(written[section]) ^ expected_keys),
        )


def test_init_undetected_values_equal_specs_defaults(tmp_path):
    """Every key detection has no answer for carries the SPECS default."""
    import io
    import json
    import subprocess

    from vsdd.cli import dispatch

    root = tmp_path / "defaults"
    root.mkdir()
    subprocess.run(["git", "init", "-q", "-b", "main", str(root)], check=True)
    subprocess.run(
        ["git", "config", "user.email", "t@e.st"], cwd=root, check=True, capture_output=True
    )
    subprocess.run(
        ["git", "config", "user.name", "T"], cwd=root, check=True, capture_output=True
    )
    (root / "README.md").write_text("# t\n", encoding="utf-8")
    subprocess.run(["git", "add", "-A"], cwd=root, check=True, capture_output=True)
    subprocess.run(
        ["git", "commit", "-qm", "x"], cwd=root, check=True, capture_output=True
    )
    out, err = io.StringIO(), io.StringIO()
    code = dispatch.main(
        ["-C", str(root), "init", "--json"], stdout=out, stderr=err, stdin_tty=True
    )
    assert code == 0, err.getvalue()
    payload = json.loads(out.getvalue())
    undetected = payload["undetected"]
    assert undetected  # commands are undetected at slice depth
    written = _written_sections(root)
    for dotted in undetected:
        section, key = dotted.split(".", 1)
        assert written[section][key] == SPECS[section][key][0], dotted
    # Every key init did not detect carries the SPECS default, and every key
    # it detected carries the detected value: a default restated by hand
    # (say agents_per_run = 999) or a dropped key fails here (finding
    # S3R-state-core-1).
    detected = payload["detected"]
    for section, keys in SPECS.items():
        for key in keys:
            if section == "hooks" and key == "rules":
                continue  # the composite the loader fills
            if key == "protected_paths":
                continue  # never written: the loader derives it from [release]
            dotted = f"{section}.{key}"
            if dotted in detected:
                assert written[section][key] == detected[dotted], dotted
            else:
                assert written[section][key] == SPECS[section][key][0], dotted
    # A detected key carries the detected value, not the default: no remote
    # means mode local (detected), while a wholly undetected commands table
    # holds the loader's empty strings.
    assert written["delivery"]["mode"] == "local"  # no remote: detected, stricter
    assert written["commands"] == {key: "" for key in SPECS["commands"]}


def test_editing_release_changelog_moves_protected_paths(tmp_path):
    """Editing release.changelog moves the computed protected_paths: init
    writes no frozen copy (finding AU-T2-15, through a real init'd project)."""
    from vsdd.config import load

    root = make_project(tmp_path / "pp")
    loader = load(root / ".factory" / "factory.toml")
    assert "CHANGELOG.md" in loader.get("delivery", "protected_paths")
    from ._repo import set_keys

    set_keys(root, "release", changelog="RELEASES.md")
    edited = load(root / ".factory" / "factory.toml")
    protected = edited.get("delivery", "protected_paths")
    assert "RELEASES.md" in protected
    assert "CHANGELOG.md" not in protected
