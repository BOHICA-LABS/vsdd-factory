"""The project configuration loader: the full key table of state.md 2.12."""

from pathlib import Path

import pytest

from vsdd.config import ConfigError, load, loads

EMPTY = "schema = 1\n"


class TestDefaults:
    def test_minimal_file_takes_design_defaults(self):
        cfg = loads(EMPTY)
        assert cfg.get("limits", "agents_per_run") == 900
        assert cfg.get("limits", "agents_per_story") == 40
        assert cfg.get("convergence", "passes") == 3
        assert cfg.get("spikes", "per_step") == 2
        assert cfg.get("delivery", "protected_paths") == [
            ".github/**", ".env*", "CODEOWNERS", "CHANGELOG.md",
        ]
        assert cfg.get("state", "push") is True
        assert cfg.get("hooks", "main_session_wall") is True
        assert cfg.command("test") == ""

    def test_missing_commands_are_a_state_not_an_error(self):
        cfg = loads(EMPTY)
        assert cfg.command("build") == ""
        assert cfg.command("test_report") == ""

    def test_get_sections(self):
        cfg = loads(EMPTY)
        assert cfg.get("branches")["integration"] == "develop"


class TestUnknownKeys:
    def test_unknown_key_lists_the_neighbours(self):
        with pytest.raises(ConfigError, match=r"\[limits\]\.agents: unknown key"):
            loads(EMPTY + "[limits]\nagents = 10\n")

    def test_unknown_section(self):
        with pytest.raises(ConfigError, match=r"unknown section \[commandss\]"):
            loads(EMPTY + "[commandss]\nbuild = 'x'\n")

    def test_wrong_type(self):
        with pytest.raises(ConfigError, match="must be an integer, got string"):
            loads(EMPTY + '[limits]\ngate_minutes = "long"\n')

    def test_boolean_is_not_an_integer(self):
        with pytest.raises(ConfigError, match="must be an integer, got boolean"):
            loads(EMPTY + "[limits]\ngate_minutes = true\n")

    def test_enum_violation(self):
        with pytest.raises(ConfigError, match="one of pr\\|local"):
            loads(EMPTY + '[delivery]\nmode = "ssh"\n')

    def test_schema_key_required(self):
        with pytest.raises(ConfigError, match="schema key is missing"):
            loads("")

    def test_schema_other_than_one(self):
        with pytest.raises(ConfigError, match="schema must be the integer 1"):
            loads("schema = 2\n")

    def test_schema_float_is_refused(self):
        with pytest.raises(ConfigError, match="schema must be the integer 1"):
            loads("schema = 1.0\n")

    def test_invalid_toml(self):
        with pytest.raises(ConfigError, match="invalid TOML"):
            loads("schema = \n= = =")


class TestEnumsAccepted:
    def test_every_enum_choice_passes(self):
        cfg = loads(
            EMPTY
            + '[delivery]\nmode = "local"\nci = "none"\nmerge = "human"\ndependency_changes = "auto"\n'
            + '[project]\nlayout = "hub"\n'
            + "[stops]\nplan = \"always\"\n"
            + '[notify]\nchannel = "command"\n'
        )
        assert cfg.get("delivery", "mode") == "local"
        assert cfg.get("delivery", "ci") == "none"
        assert cfg.get("project", "layout") == "hub"


class TestCompositeKeys:
    def test_commands_env_table(self):
        cfg = loads(EMPTY + '[commands.env]\nSCCACHE_DIR = "~/.cache/sccache"\n')
        assert cfg.get("commands", "env") == {"SCCACHE_DIR": "~/.cache/sccache"}

    def test_env_value_must_be_a_string(self):
        with pytest.raises(ConfigError, match="must be a string"):
            loads(EMPTY + "[commands.env]\nN = 3\n")

    def test_kill_rate_overrides_and_keeps_the_rest(self):
        cfg = loads(EMPTY + "[mutation.kill_rate]\ncritical = 1.0\n")
        rates = cfg.get("mutation", "kill_rate")
        assert rates["critical"] == 1.0
        assert rates["low"] == 0.70

    def test_kill_rate_rejects_unknown_criticality(self):
        with pytest.raises(ConfigError, match="unknown criticality"):
            loads(EMPTY + "[mutation.kill_rate]\nexistential = 0.5\n")

    def test_auto_allow_scope_choices(self):
        cfg = loads(EMPTY + "[hooks.rules.auto-allow]\nscope = \"extended\"\n")
        assert cfg.get("hooks", "rules")["auto-allow"]["scope"] == "extended"

    def test_auto_allow_scope_violation(self):
        """A bad [hooks] value makes only that rule fall back to its defaults
        (hooks.md 2.6): the problem is reported per rule, the file still loads,
        and the other rules keep their settings."""
        cfg = loads(
            EMPTY
            + '[hooks.rules.auto-allow]\nscope = "everything"\n'
            + '[hooks.rules.tests-first]\nstrength = "advise"\n'
        )
        assert cfg.get("hooks", "rules").get("auto-allow") is None
        assert cfg.hook_problems["auto-allow"]
        assert any("off|core|extended" in message for message in cfg.hook_problems["auto-allow"])
        # the other rule keeps its setting
        assert cfg.get("hooks", "rules")["tests-first"] == {"strength": "advise"}
        assert "tests-first" not in cfg.hook_problems

    def test_strength_must_be_a_known_choice(self):
        cfg = loads(EMPTY + '[hooks.rules.tests-first]\nstrength = "of"\n')
        assert cfg.get("hooks", "rules").get("tests-first") is None
        assert "tests-first" in cfg.hook_problems

    def test_strength_on_a_locked_rule_is_reported(self):
        cfg = loads(EMPTY + '[hooks.rules.read-walls]\nstrength = "advise"\n')
        assert "read-walls" in cfg.hook_problems
        assert any("locked" in message for message in cfg.hook_problems["read-walls"])

    def test_auto_allow_takes_no_other_key(self):
        cfg = loads(EMPTY + "[hooks.rules.auto-allow]\nscpe = \"core\"\n")
        assert "auto-allow" in cfg.hook_problems
        assert any("scpe" in message for message in cfg.hook_problems["auto-allow"])

    def test_unknown_hook_rule_tables_are_accepted(self):
        cfg = loads(EMPTY + '[hooks.rules.tests-first]\nstrength = "advise"\n')
        assert cfg.get("hooks", "rules")["tests-first"]["strength"] == "advise"

    def test_version_files_entries(self):
        cfg = loads(EMPTY + '[[release.version_files]]\npath = "Cargo.toml"\nkey = "package.version"\n')
        assert cfg.get("release", "version_files") == [{"path": "Cargo.toml", "key": "package.version"}]

    def test_caps_free_form_and_waived(self):
        cfg = loads(EMPTY + "[caps]\nstory = 20000\nwaived = [\"specs/architecture/ADR-052.md\"]\n")
        assert cfg.get("caps")["story"] == 20000
        assert cfg.get("caps")["waived"] == ["specs/architecture/ADR-052.md"]

    def test_waived_survives_a_caps_table_without_it(self):
        cfg = loads(EMPTY + "[caps]\nstory = 20000\n")
        assert cfg.get("caps")["waived"] == []

    def test_discover_source(self):
        cfg = loads(EMPTY + '[[discover.source]]\nname = "github-issues"\ncommand = "gh issue list"\n')
        assert cfg.get("discover", "source") == [{"name": "github-issues", "command": "gh issue list"}]

    def test_discover_source_requires_name_and_command(self):
        with pytest.raises(ConfigError, match="command must be a non-empty string"):
            loads(EMPTY + '[[discover.source]]\nname = "github-issues"\n')


HUB = EMPTY + '[project]\nlayout = "hub"\n'


class TestRepos:
    def test_repo_entry_with_overrides(self):
        text = (
            HUB
            + '[[repo]]\nname = "api-server"\npath = "repos/api-server"\nurl = "git@example:acme/api-server.git"\n'
            + "[repo.commands]\ntest = \"pnpm -C api test\"\n"
        )
        cfg = loads(text)
        (repo,) = cfg.get("repo")
        assert repo["name"] == "api-server"
        assert repo["commands"]["test"] == "pnpm -C api test"

    def test_repo_override_unknown_key_is_an_error(self):
        text = HUB + '[[repo]]\nname = "api"\npath = "repos/api"\nurl = "u"\n[repo.commands]\ntst = "pnpm test"\n'
        with pytest.raises(ConfigError, match=r"\[\[repo\]\]\[0\]\.commands\.tst: unknown key"):
            loads(text)

    def test_repo_release_version_files(self):
        """Blueprint 4.12: a [[repo]] may carry its own [release]."""
        text = (
            HUB
            + '[[repo]]\nname = "api"\npath = "repos/api"\nurl = "u"\n'
            + '[[repo.release.version_files]]\npath = "repos/api/Cargo.toml"\nkey = "package.version"\n'
        )
        (repo,) = loads(text).get("repo")
        assert repo["release"]["version_files"] == [
            {"path": "repos/api/Cargo.toml", "key": "package.version"}
        ]
        assert repo["release"]["tag_format"] == "v{version}"  # the default survives

    def test_repo_commands_env(self):
        text = (
            HUB
            + '[[repo]]\nname = "api"\npath = "repos/api"\nurl = "u"\n'
            + '[repo.commands.env]\nSCCACHE_DIR = "~/.cache"\n'
        )
        (repo,) = loads(text).get("repo")
        assert repo["commands"]["env"] == {"SCCACHE_DIR": "~/.cache"}

    def test_repo_delivery_protected_paths_and_range(self):
        text = (
            HUB
            + '[[repo]]\nname = "api"\npath = "repos/api"\nurl = "u"\n'
            + "[repo.delivery]\nci_wait_minutes = 15\nprotected_paths = [\"repos/api/.github/**\"]\n"
        )
        (repo,) = loads(text).get("repo")
        assert repo["delivery"]["protected_paths"] == ["repos/api/.github/**"]
        assert repo["delivery"]["ci_wait_minutes"] == 15

    def test_repo_release_version_files_bad_entry_is_an_error(self):
        text = (
            HUB
            + '[[repo]]\nname = "api"\npath = "repos/api"\nurl = "u"\n'
            + '[[repo.release.version_files]]\nkey = "package.version"\n'
        )
        with pytest.raises(ConfigError, match=r"path must be a non-empty string"):
            loads(text)

    def test_repo_range_violation_through_the_shared_route(self):
        text = (
            HUB
            + '[[repo]]\nname = "api"\npath = "repos/api"\nurl = "u"\n'
            + "[repo.delivery]\nci_wait_minutes = 0\n"
        )
        with pytest.raises(ConfigError, match="at least 1"):
            loads(text)

    def test_repo_unknown_top_level_key(self):
        text = HUB + '[[repo]]\nname = "api"\npath = "p"\nurl = "u"\npathh = "x"\n'
        with pytest.raises(ConfigError, match=r"pathh: unknown key"):
            loads(text)

    def test_repo_requires_hub_layout(self):
        text = EMPTY + '[[repo]]\nname = "api"\npath = "repos/api"\nurl = "u"\n'
        with pytest.raises(ConfigError, match="layout = \\\\\"hub\\\\\"|need project.layout"):
            loads(text)

    def test_duplicate_repo_names_are_refused(self):
        text = (
            HUB
            + '[[repo]]\nname = "api"\npath = "a"\nurl = "u"\n'
            + '[[repo]]\nname = "api"\npath = "b"\nurl = "u"\n'
        )
        with pytest.raises(ConfigError, match="duplicate repo name 'api'"):
            loads(text)


class TestClosedSpecs:
    """[[component]], [[reference]], [[repo]] and [discover] accept exactly the
    keys the state.md 2.12 example shows; anything else is an error (hooks.md
    2.6: a misspelt component command key would silently weaken the green gate)."""

    def test_component_accepts_the_example(self):
        text = EMPTY + '[[component]]\nname = "web"\npaths = ["web/**"]\ncommands = { test = "pnpm -C web test" }\n'
        (component,) = loads(text).get("component")
        assert component == {"name": "web", "paths": ["web/**"], "commands": {"test": "pnpm -C web test"}}

    def test_component_unknown_key(self):
        with pytest.raises(ConfigError, match=r"\[\[component\]\]\[0\]\.pahts: unknown key"):
            loads(EMPTY + '[[component]]\nname = "web"\npahts = ["x"]\n')

    def test_component_wrong_type(self):
        with pytest.raises(ConfigError, match=r"paths must be a list of strings"):
            loads(EMPTY + '[[component]]\nname = "web"\npaths = 5\n')

    def test_component_unknown_command_key(self):
        with pytest.raises(ConfigError, match=r"commands\.tset: unknown key"):
            loads(EMPTY + '[[component]]\nname = "web"\ncommands = { tset = "true" }\n')

    def test_component_command_must_be_a_string(self):
        with pytest.raises(ConfigError, match=r"commands\.test must be a string"):
            loads(EMPTY + '[[component]]\nname = "web"\ncommands = { test = true }\n')

    def test_reference_accepts_the_example(self):
        text = EMPTY + '[[reference]]\nname = "superpowers"\nurl = "https://example.com/x"\n'
        (reference,) = loads(text).get("reference")
        assert reference == {"name": "superpowers", "url": "https://example.com/x"}

    def test_reference_unknown_key(self):
        with pytest.raises(ConfigError, match=r"\[\[reference\]\]\[0\]\.nmae: unknown key"):
            loads(EMPTY + '[[reference]]\nnmae = "superpowers"\nurl = "https://x"\n')

    def test_reference_requires_url(self):
        with pytest.raises(ConfigError, match="url must be a non-empty string"):
            loads(EMPTY + '[[reference]]\nname = "superpowers"\n')

    def test_discover_unknown_key(self):
        with pytest.raises(ConfigError, match=r"\[discover\]\.sources: unknown key"):
            loads(EMPTY + '[discover]\nsources = ["github"]\n')

    def test_discover_source_unknown_key(self):
        text = EMPTY + '[[discover.source]]\nname = "gh"\ncommand = "gh issue list"\nkinds = ["a"]\n'
        with pytest.raises(ConfigError, match=r"kinds: unknown key"):
            loads(text)


class TestRanges:
    """An out-of-range value never loosens a gate silently (U2-R15)."""

    def test_zero_passes(self):
        with pytest.raises(ConfigError, match="at least 1"):
            loads(EMPTY + "[convergence]\npasses = 0\n")

    def test_negative_step_attempts(self):
        with pytest.raises(ConfigError, match="at least 1"):
            loads(EMPTY + "[limits]\nstep_attempts = -1\n")

    def test_mean_min_above_one(self):
        with pytest.raises(ConfigError, match="between 0 and 1"):
            loads(EMPTY + "[holdout]\nmean_min = 5\n")

    def test_stddev_max_above_one(self):
        with pytest.raises(ConfigError, match="between 0 and 1"):
            loads(EMPTY + "[holdout]\nstddev_max = 2\n")

    def test_kill_rate_above_one(self):
        with pytest.raises(ConfigError, match="between 0 and 1"):
            loads(EMPTY + "[mutation.kill_rate]\ncritical = 2\n")

    def test_spike_minutes_zero(self):
        with pytest.raises(ConfigError, match="at least 1"):
            loads(EMPTY + "[spikes]\nminutes = 0\n")

    @pytest.mark.xfail(
        reason="[caps] names and the twice-the-default bound are the artifact "
        "registry's (C2); recorded as a deferred check, not skipped silently"
    )
    def test_caps_bound_check_is_c2(self):
        with pytest.raises(ConfigError):
            loads(EMPTY + "[caps]\nstory = -5\n")


class TestPreludeLimits:
    def test_mapping_names(self):
        limits = loads(EMPTY).prelude_limits()
        assert set(limits) == {
            "agents", "story_agents", "stories_per_run", "attempts", "delivery_rounds",
            "gate_minutes", "ci_minutes", "passes", "rounds",
            "spikes_per_step", "spikes_per_run", "spike_minutes",
        }

    def test_mapping_follows_the_file(self):
        cfg = loads(EMPTY + "[limits]\nagents_per_story = 12\n" + "[spikes]\nminutes = 5\n")
        assert cfg.prelude_limits()["story_agents"] == 12
        assert cfg.prelude_limits()["spike_minutes"] == 5


class TestFileLoading:
    def test_load_from_disk(self, tmp_path: Path):
        path = tmp_path / "factory.toml"
        path.write_text("schema = 1\n[project]\nname = \"acme-api\"\nlanguages = [\"rust\"]\n", encoding="utf-8")
        cfg = load(path)
        assert cfg.get("project", "name") == "acme-api"
        assert cfg.path == path

    def test_load_missing_file(self, tmp_path: Path):
        with pytest.raises(ConfigError, match="cannot read"):
            load(tmp_path / "absent.toml")

    def test_the_design_example_loads(self):
        """The whole key list of state.md 2.12, exactly as the example shows."""
        example = '''
schema = 1
[project]
name = "acme-api"
languages = ["rust"]
ui = false
layout = "single"
[branches]
integration = "develop"
release = "main"
protected = ["main"]
story = "story/{id}"
spike = "spike/{name}"
[commands]
build = "cargo build --workspace"
test = "cargo nextest run --workspace --profile ci"
test_report = "junit:target/nextest/ci/junit.xml"
lint = "cargo clippy --workspace --all-targets -- -D warnings"
format_check = "cargo fmt --check --all"
[paths]
tests = ["tests/**", "crates/*/tests/**"]
source = ["src/**", "crates/*/src/**", "build.rs"]
[delivery]
mode = "pr"
ci = "required"
merge = "auto"
merge_strategy = "squash"
dependency_changes = "human"
ci_wait_minutes = 30
[convergence]
passes = 3
rounds = 3
[limits]
agents_per_run = 900
agents_per_story = 40
stories_per_wave_run = 24
step_attempts = 2
delivery_rounds = 3
gate_minutes = 60
parallel_gates = 4
failed_runs_before_stuck = 2
[spikes]
per_step = 2
per_run = 6
per_day = 20
minutes = 20
max_open = 3
retain_days = 14
[holdout]
mean_min = 0.85
must_pass_min = 0.6
stddev_max = 0.15
rotation = 0.8
batch = 12
[mutation.kill_rate]
critical = 0.95
high = 0.90
medium = 0.80
low = 0.70
[stops]
plan = "auto"
plan_ceiling_agents = 600
[evidence]
wave_demo = false
final_demo = false
[maintain]
sweeps = []
fixes_per_run = 10
[release]
changelog = "CHANGELOG.md"
tag_format = "v{version}"
github_release = true
[state]
push = true
lock_stale_minutes = 20
retain_runs_days = 30
prune_closed_cycles = true
[notify]
channel = "none"
command = ""
[hooks]
main_session_wall = true
[hooks.rules.auto-allow]
scope = "off"
'''
        cfg = loads(example)
        assert cfg.get("project", "name") == "acme-api"
        assert cfg.get("hooks", "rules")["auto-allow"]["scope"] == "off"
        assert cfg.prelude_limits()["ci_minutes"] == 30
