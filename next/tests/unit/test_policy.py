"""The C2 policy tables: each shipped table says what the design requires.

One test per design fact, tied to the row that must carry it at slice depth
(the main session, the clerk, the test-writer, the implementer and the
adversary; the story artifact kind; the classes a story touches). The
remaining rows are carried as the design gives them and asserted only where a
whole-table invariant (every row denies runs, the group and kind vocabularies
agree across the tables) covers them.
"""

from __future__ import annotations

import tomllib
from pathlib import Path

import pytest

from vsdd import policy

ARTIFACTS = policy.load("artifacts")
CLASSES = policy.load("classes")
WALLS = policy.load("walls")

# The HUMAN_ONLY and GOVERNED constants have one home, the design's tables/callers.toml, until the
# hook rules ship (H6) and vsdd/hooks/model.py takes them; no test or check repeats the values.
_SPEC_CALLERS_PATH = (
    Path(__file__).resolve().parents[2].parent / "rewrite" / "spec" / "tables" / "callers.toml"
)
_SPEC_CALLERS = tomllib.loads(_SPEC_CALLERS_PATH.read_text())
HUMAN_ONLY = set(_SPEC_CALLERS["HUMAN_ONLY"])
GOVERNED = set(_SPEC_CALLERS["GOVERNED"])

CALLER_ROWS = {
    name: row
    for name, row in WALLS.items()
    if isinstance(row, dict) and name not in ("callers", "reveals")
}
AGENT_ROWS = {name: row for name, row in CALLER_ROWS.items() if name.startswith("vsdd-factory:")}
GROUPS = WALLS["callers"]
REVEALS = WALLS["reveals"]
PRODUCT_CLASSES = CLASSES["_product"]["precedence"]
COMPUTED_CLASSES = {"own-worktree", "project", "tmp", "outside"}
CLASS_NAMES = {name for name in CLASSES if name != "_product"} | COMPUTED_CLASSES
STORY_KINDS = {row["id"]: row for row in ARTIFACTS["story-kind"]}
MAIN = WALLS["main"]
CLERK = WALLS["vsdd-factory:clerk"]


def granted_classes(row: dict, key: str) -> set[str]:
    """The class names a row's read_deny/read_allow/write list names."""
    return {grant.split(":")[0] for grant in row.get(key, [])}


# ---------------------------------------------------------------------------
# walls: the slice rows


class TestMainRow:
    def test_every_key_is_pinned(self):
        # The full row: read_deny, write, dispatch, commands, vsdd and relaxed_by.
        assert set(MAIN) == {"read_deny", "write", "dispatch", "commands", "vsdd", "relaxed_by"}
        assert set(MAIN["read_deny"]) == {"holdout", "runs"}
        assert set(MAIN["write"]) == {"brief", "deferred", "project"}
        assert MAIN["dispatch"] == ["*"]
        assert MAIN["commands"] == ["*"]
        assert set(MAIN["vsdd"]) == {"read", "author", "operator"}
        assert MAIN["relaxed_by"] == "hooks.main_session_wall"


class TestOtherRow:
    def test_every_key_is_pinned(self):
        # Any subagent the main session dispatches that no row lists falls to [other]: the
        # strictest treatment, pinned cell for cell (C2-R2).
        row = WALLS["other"]
        assert set(row) == {"read_deny", "write"}
        assert set(row["read_deny"]) == {"holdout", "reviews", "runs"}
        assert row["write"] == ["project"]

    def test_unlisted_dispatched_agents_read_holdout_only_through_the_other_row(self):
        # The holdout-reader invariant is stated over the vsdd-factory:* rows; the [other] row
        # is the twin for a dispatched agent type no row lists, so it denies holdout too.
        assert "holdout" in WALLS["other"]["read_deny"]


class TestClerkRow:
    def test_every_key_is_pinned(self):
        assert set(CLERK) == {"tool_only", "read_deny", "write", "dispatch", "vsdd"}
        assert CLERK["tool_only"] is True
        assert set(CLERK["read_deny"]) == {"holdout", "reviews", "runs", "outside"}
        assert CLERK["write"] == []
        assert CLERK["dispatch"] == []
        assert set(CLERK["vsdd"]) == {"read", "pipeline", "measure", "route", "env"}

    def test_holds_every_gate_group_alone(self):
        # measure = every gate (gate wave --settle and gate plan --story move
        # state); no other row holds it, so only the clerk may.
        assert "measure" in CLERK["vsdd"]
        holders = {name for name, row in CALLER_ROWS.items() if "measure" in row.get("vsdd", [])}
        assert holders == {"vsdd-factory:clerk"}

    def test_reads_no_pipeline_output(self):
        # The clerk reads no pipeline output: its row denies runs, reviews and
        # holdout like any other, and findings arrive through vsdd review show.
        assert {"runs", "reviews", "holdout"} <= set(CLERK["read_deny"])


class TestTestWriterRow:
    row = WALLS["vsdd-factory:test-writer"]

    def test_every_key_is_pinned(self):
        assert set(self.row) == {"read_deny", "write", "story_only", "dispatch", "commands", "vsdd"}
        assert set(self.row["read_deny"]) == {"holdout", "runs"}
        assert set(self.row["write"]) == {"tests:frozen", "manifests", "source:before-red"}
        assert self.row["story_only"] is True
        assert self.row["dispatch"] == ["vsdd-factory:spike-runner"]
        assert set(self.row["commands"]) == {"git-commit", "git-read"}
        assert set(self.row["vsdd"]) == {"read", "story-gate", "refreeze"}

    def test_alone_refreezes_the_red_gate(self):
        assert "refreeze" in self.row["vsdd"]
        holders = {n for n, r in CALLER_ROWS.items() if "refreeze" in r.get("vsdd", [])}
        assert holders == {"vsdd-factory:test-writer"}


class TestImplementerRow:
    row = WALLS["vsdd-factory:implementer"]

    def test_every_key_is_pinned(self):
        assert set(self.row) == {"read_deny", "write", "story_only", "dispatch", "commands", "vsdd"}
        assert set(self.row["read_deny"]) == {"holdout", "runs"}
        assert set(self.row["write"]) == {"source", "manifests"}
        assert "tests" not in granted_classes(self.row, "write")
        assert self.row["story_only"] is True
        assert self.row["dispatch"] == ["vsdd-factory:spike-runner"]
        assert set(self.row["commands"]) == {"git-commit", "git-read"}
        assert set(self.row["vsdd"]) == {"read", "story-gate"}


class TestAdversaryRow:
    row = WALLS["vsdd-factory:adversary"]

    def test_every_key_is_pinned(self):
        assert set(self.row) == {"read_deny", "write", "dispatch"}
        assert set(self.row["read_deny"]) == {"holdout", "reviews", "runs", "outside"}
        assert self.row["write"] == ["reviews:create"]
        assert self.row["dispatch"] == ["vsdd-factory:spike-runner"]


# ---------------------------------------------------------------------------
# walls: whole-table invariants


class TestWallTableInvariants:
    def test_every_row_denies_runs(self):
        denied = {name for name, row in CALLER_ROWS.items() if "runs" in row.get("read_deny", [])}
        assert denied == set(CALLER_ROWS)

    def test_shell_less_rows_deny_outside(self):
        for name in ("vsdd-factory:researcher", "vsdd-factory:security-reviewer",
                     "vsdd-factory:holdout-evaluator", "vsdd-factory:scenario-reviewer",
                     "vsdd-factory:clerk", "vsdd-factory:spike-runner",
                     "vsdd-factory:code-reviewer", "vsdd-factory:adversary"):
            assert "outside" in WALLS[name]["read_deny"], name

    def test_every_reviewer_row_denies_reviews(self):
        # A reviewer reads no earlier review: every reviewer-tier row denies the reviews class
        # (C2-R2), so a pass is written without an earlier pass's text at hand.
        for name in ("vsdd-factory:adversary", "vsdd-factory:code-reviewer",
                     "vsdd-factory:security-reviewer", "vsdd-factory:ui-reviewer",
                     "vsdd-factory:holdout-evaluator", "vsdd-factory:scenario-reviewer"):
            assert "reviews" in WALLS[name]["read_deny"], name

    def test_the_non_list_reveals_are_pinned_exactly(self):
        # The reads that are no `list <kind>`: emptied, narrowed or reordered entries (C2-R2)
        # change what review show, refs and coverage reveal to a walled caller.
        assert {k: v for k, v in REVEALS.items() if not k.startswith("list ")} == {
            "refs --include-walled": ["holdout", "reviews"],
            "coverage --include-walled": ["holdout"],
            "review show": ["reviews"],
            "review show --include-walled": ["reviews", "holdout"],
        }

    def test_tool_only_rows_deny_outside(self):
        # A vsdd-only shell still reaches nothing outside the project, the
        # plugin and the temporary directory.
        for name, row in CALLER_ROWS.items():
            if row.get("tool_only"):
                assert "outside" in row["read_deny"], name

    def test_holdout_is_readable_only_by_the_scenario_roles(self):
        # The builders' holdout wall, the premise of the walls: an agent reads
        # hidden scenarios only when its row leaves holdout out of read_deny.
        # The main session is not among them; its relaxation is the [hooks] key.
        readers = {
            name
            for name, row in CALLER_ROWS.items()
            if name.startswith("vsdd-factory:") and "holdout" not in row.get("read_deny", [])
        }
        assert readers == {
            "vsdd-factory:product-owner",
            "vsdd-factory:scenario-reviewer",
            "vsdd-factory:holdout-evaluator",
        }
        assert "holdout" in MAIN["read_deny"]
        assert MAIN["relaxed_by"] == "hooks.main_session_wall"

    def test_tool_only_rows_are_the_clerk_and_the_four_planners(self):
        tool_only = {name for name, row in CALLER_ROWS.items() if row.get("tool_only")}
        assert tool_only == {
            "vsdd-factory:clerk",
            "vsdd-factory:product-owner",
            "vsdd-factory:architect",
            "vsdd-factory:ux-designer",
            "vsdd-factory:story-writer",
        }

    def test_every_agent_dispatches_only_the_spike_runner(self):
        for name, row in AGENT_ROWS.items():
            if name in ("vsdd-factory:spike-runner", "vsdd-factory:clerk"):
                assert row.get("dispatch") == [], name
            else:
                assert row.get("dispatch") == ["vsdd-factory:spike-runner"], name

    def test_relaxed_by_on_the_main_row_only(self):
        carriers = {name for name, row in CALLER_ROWS.items() if "relaxed_by" in row}
        assert carriers == {"main"}

    def test_row_grants_name_defined_classes(self):
        for name, row in CALLER_ROWS.items():
            for key in ("read_deny", "read_allow", "write"):
                unknown = granted_classes(row, key) - CLASS_NAMES
                assert not unknown, f"{name}.{key}: {sorted(unknown)}"

    def test_row_vsdd_keys_name_defined_groups(self):
        for name, row in CALLER_ROWS.items():
            unknown = set(row.get("vsdd", [])) - set(GROUPS)
            assert not unknown, f"{name}: {sorted(unknown)}"

    def test_row_commands_name_defined_command_families(self):
        families = {"git-commit", "git-read", "git-push", "git-integrate", "git-worktree",
                    "gh-pr", "git-other", "*"}
        for name, row in CALLER_ROWS.items():
            unknown = set(row.get("commands", [])) - families
            assert not unknown, f"{name}: {sorted(unknown)}"

    def test_the_callers_groups_are_pinned_member_for_member(self):
        # The subcommand groups a row may hold, pinned so a drift in a group's
        # members is a conscious change to the walls and not a silent one.
        expected = {
            "read": ["status", "list", "refs", "coverage", "path", "validate", "config", "explain",
                     "rules", "review show", "--version"],
            "measure": ["gate"],
            "refreeze": ["gate red --refreeze"],
            "story-gate": ["gate red", "gate green", "gate demo"],
            "spec-check": ["gate spec --only"],
            "admit": ["gate plan --story"],
            "route": ["feature route"],
            "pipeline": ["survey", "checkpoint", "plan schedule", "story", "release", "idea",
                         "report", "discover", "repos split", "status --fix"],
            "spike": ["spike"],
            "env": ["env"],
            "author": ["new", "note", "defer", "retire", "driver", "lock release", "review prepare",
                       "init", "import --dry-run", "checkpoint brief", "status --fix"],
            "operator": ["doctor", "events", "obs", "export", "render", "hooks", "repos scan"],
        }
        assert GROUPS == expected

    def test_version_reveals_nothing_so_read_carries_it(self):
        # P3SEAM-21: a workflow step (smoke probe 6) runs `vsdd --version`, so
        # the read group every caller may decline to hold must contain it.
        assert "--version" in GROUPS["read"]
        assert "--version" not in GROUPS["operator"]

    def test_person_only_subcommands_are_in_no_group(self):
        # The HUMAN_ONLY constant of the tool's code (one copy, in the design's
        # tables/callers.toml), matched on the longest subcommand; a shorter
        # group entry that prefixes one is still a distinct subcommand (author
        # holds "lock release" and "import --dry-run", neither of which is
        # person-only).
        granted = {sub for members in GROUPS.values() for sub in members}
        assert not HUMAN_ONLY & granted

    def test_governed_subcommands_name_real_routes(self):
        # The GOVERNED subcommands are the routes onto the integration and
        # release branches (merge-guard): each must be reachable through a
        # granted group entry, or the governance would bite on nothing.
        granted = {sub for members in GROUPS.values() for sub in members}
        for gov in GOVERNED:
            assert any(entry == gov or gov.startswith(entry) for entry in granted), gov
        assert not GOVERNED & HUMAN_ONLY


# ---------------------------------------------------------------------------
# artifacts: the story kind row, the whole registry


class TestStoryArtifactKind:
    row = ARTIFACTS["story"]

    def test_path_identifier_and_cap(self):
        assert self.row["glob"] == ".factory/stories/E-*/S-*.md"
        assert self.row["id"] == r"^S-(\d+)\.(\d{2})$"
        assert self.row["dir"] == "E-{1}"
        assert self.row["max_bytes"] == 16000

    def test_writer_lock_and_heading(self):
        assert self.row["writer"] == "story-writer"
        assert self.row["lock"] == "done"
        assert self.row["headings"] == ["Acceptance criteria"]

    def test_volatile_citations_forbidden_and_tables_checked(self):
        assert any("rs|py|ts" in pattern for pattern in self.row["forbid"])
        assert self.row["tables"] is True

    def test_nineteen_frontmatter_keys_every_one_typed(self):
        # state.md 2.5's keys with the optional stage-method key split into
        # impl_method and tests_method, so one key names one stage; every key
        # carries a type the artifacts rule can check, "?" marking an optional
        # key and the descriptions living in comments.
        frontmatter = self.row["frontmatter"]
        assert len(frontmatter) == 19
        assert all(value for value in frontmatter.values()), frontmatter
        assert set(frontmatter["kind"]) == {"feature", "fix", "port", "facade", "docs", "chore", "harness"}

    def test_story_frontmatter_types_and_optional_keys_are_pinned(self):
        frontmatter = self.row["frontmatter"]
        optional = {k for k, v in frontmatter.items() if isinstance(v, str) and v.endswith("?")}
        assert optional == {
            "impl_method", "tests_method", "depends_on", "properties", "points",
            "security", "ui", "demo", "screens", "repo", "source",
        }
        assert frontmatter["id"] == "str"
        assert frontmatter["title"] == "str"
        assert frontmatter["cycle"] == "str"
        assert frontmatter["points"] == "int?"
        assert frontmatter["depends_on"] == "list[ref:story]?"
        assert frontmatter["screens"] == "list[ref:screen]?"
        assert frontmatter["security"] == "bool?"
        assert frontmatter["ui"] == "bool?"
        assert frontmatter["demo"] == "bool?"
        assert frontmatter["contracts"] == "list[ref:contract]"
        assert frontmatter["status"] == ["draft", "dropped"]
        assert frontmatter["priority"] == ["P0", "P1", "P2"]
        assert frontmatter["criticality"] == ["critical", "high", "medium", "low"]


class TestContractArtifactKind:
    row = ARTIFACTS["contract"]

    def test_path_identifier_cap_writer_and_lock_are_pinned(self):
        # The header of artifacts.toml promises the [contract] row is pinned (L2C-c2-b6-1):
        # its glob, cap, status enum and lock each fail the suite when they drift.
        assert self.row["glob"] == ".factory/specs/product/contracts/*/BC-*.md"
        assert self.row["id"] == r"^BC-(\d+)\.(\d{2})\.(\d{3})$"
        assert self.row["dir"] == "{1}.{2}"
        assert self.row["max_bytes"] == 8000
        assert self.row["writer"] == "product-owner"
        assert self.row["lock"] == "spec"
        assert self.row["template"] == "contract.md"

    def test_headings_are_pinned(self):
        # L2C-c2-b6-1: the artifacts rule enforces the headings cell on write, so a dropped,
        # added or reordered heading changes every contract file's shape; the header says the
        # [contract] row is pinned, so the list is pinned here cell for cell.
        assert self.row["headings"] == [
            "Behavior", "Preconditions", "Postconditions", "Invariants", "Edge cases", "Test vectors",
        ]

    def test_frontmatter_keys_are_pinned(self):
        frontmatter = self.row["frontmatter"]
        assert set(frontmatter) == {
            "id", "title", "status", "subsystem", "capability", "origin", "replaced_by", "aliases",
        }
        assert frontmatter["status"] == ["draft", "retired"]
        assert frontmatter["origin"] == ["new", "recovered"]
        assert frontmatter["subsystem"] == "ref:subsystem"
        assert frontmatter["capability"] == "ref:capability"
        assert frontmatter["replaced_by"] == "ref:contract?"
        assert frontmatter["aliases"] == "list[str]?"


class TestUnknownWrites:
    def test_a_write_matching_no_kind_is_blocked(self):
        assert ARTIFACTS["_unknown"]["policy"] == "block"


class TestStoryKinds:
    def test_seven_kinds(self):
        assert set(STORY_KINDS) == {"feature", "fix", "port", "facade", "docs", "chore", "harness"}

    def test_red_gate_kinds(self):
        red = {kind for kind, row in STORY_KINDS.items() if row["red_gate"]}
        assert red == {"feature", "fix", "port"}

    def test_feature_stages(self):
        row = STORY_KINDS["feature"]
        assert (row["tests_agent"], row["tests_method"]) == ("test-writer", "failing-tests")
        assert (row["implement_agent"], row["implement_method"]) == ("implementer", "tdd")

    def test_fix_stages(self):
        row = STORY_KINDS["fix"]
        assert (row["tests_agent"], row["tests_method"]) == ("test-writer", "regression-test")
        assert (row["implement_agent"], row["implement_method"]) == ("implementer", "fix-findings")

    def test_docs_kind_implementer_is_the_technical_writer(self):
        row = STORY_KINDS["docs"]
        assert row["red_gate"] is False
        assert (row["implement_agent"], row["implement_method"]) == ("technical-writer", "docs")

    def test_chore_default_method_is_the_scaffold(self):
        row = STORY_KINDS["chore"]
        assert (row["implement_agent"], row["implement_method"]) == ("devops", "repo-setup")

    def test_all_seven_kinds_stage_cells_and_red_gate_are_pinned(self):
        # L2C-c2-b6-1: every row pinned cell for cell, so a stage agent flipping (harness:
        # verifier to implementer) or a red-gate flag changing fails here.
        expected = {
            "feature": ("test-writer", "failing-tests", "implementer", "tdd", True),
            "fix": ("test-writer", "regression-test", "implementer", "fix-findings", True),
            "port": ("test-writer", "equivalence-tests", "implementer", "port-translation", True),
            "facade": ("", "", "implementer", "service-clone", False),
            "docs": ("", "", "technical-writer", "docs", False),
            "chore": ("", "", "devops", "repo-setup", False),
            "harness": ("", "", "verifier", "proofs", False),
        }
        assert set(STORY_KINDS) == set(expected)
        for kind, (tests_agent, tests_method, impl_agent, impl_method, red_gate) in expected.items():
            row = STORY_KINDS[kind]
            assert (row["tests_agent"], row["tests_method"]) == (tests_agent, tests_method), kind
            assert (row["implement_agent"], row["implement_method"]) == (impl_agent, impl_method), kind
            assert row["red_gate"] is red_gate, kind


# ---------------------------------------------------------------------------
# classes: the classes a story touches and the product precedence


class TestPathClasses:
    def test_product_precedence_orders_tests_before_source(self):
        # src/parser/tests.rs is tests even though src/** is a source glob.
        assert PRODUCT_CLASSES == ["tests", "harness", "ci", "repo-config", "manifests", "docs", "source"]
        assert PRODUCT_CLASSES.index("tests") < PRODUCT_CLASSES.index("source")

    def test_product_classes_come_from_the_project_configuration(self):
        # The [paths] key of each class is "paths." plus the class name with
        # its hyphens turned into underscores (repo-config -> paths.repo_config).
        for name in PRODUCT_CLASSES:
            assert CLASSES[name]["from_config"] == "paths." + name.replace("-", "_"), name


def nested_classes(cls_name: str) -> set[str]:
    """The classes whose every path is also inside ``cls_name`` (classes.toml's nesting)."""
    bases = [glob.rstrip("*") for glob in CLASSES[cls_name].get("globs", [])]
    out = set()
    for other, row in CLASSES.items():
        if other in ("_product", cls_name):
            continue
        for glob in row.get("globs", []):
            if any(glob.startswith(base) and len(glob) > len(base) for base in bases):
                out.add(other)
    return out


class TestClassCoverage:
    """Walls.toml's coverage rule: a named class covers the classes nested inside it, except
    those the row's read_allow names."""

    def test_the_nesting_relation_is_the_globs_prefix_one(self):
        assert nested_classes("holdout") == {"holdout-evaluations"}
        assert nested_classes("spec-product") == {"brief"}
        assert nested_classes("spec-arch") == {"guidance"}
        assert nested_classes("research") == {"research-inputs"}
        assert nested_classes("control") == {"decisions"}
        assert nested_classes("local") == {"runs", "evidence"}

    def test_a_denied_class_covers_its_nested_classes(self):
        # Main denies holdout, so the evaluations inside holdout are denied with
        # it: the walls' holdout wall needs no second list entry.
        assert "holdout" in MAIN["read_deny"]
        assert "holdout-evaluations" not in MAIN["read_deny"]
        assert MAIN.get("read_allow", []) == []

    def test_a_write_grant_covers_its_nested_classes(self):
        # The product-owner's spec-product grant covers brief; the architect's
        # spec-arch grant covers guidance.
        po = WALLS["vsdd-factory:product-owner"]
        assert "spec-product" in po["write"] and "brief" not in po["write"]
        architect = WALLS["vsdd-factory:architect"]
        assert "spec-arch" in architect["write"] and "guidance" not in architect["write"]

    def test_read_allow_exceptions(self):
        # The code-reviewer denies spec-arch yet reads guidance; the
        # holdout-evaluator denies spec-product yet reads brief.
        cr = WALLS["vsdd-factory:code-reviewer"]
        assert "spec-arch" in cr["read_deny"] and cr["read_allow"] == ["evidence", "guidance"]
        he = WALLS["vsdd-factory:holdout-evaluator"]
        assert "spec-product" in he["read_deny"] and he["read_allow"] == ["brief"]

    def test_story_state_classes_are_tools_written(self):
        # The state classes a story touches; project code classes (tests,
        # source, docs, manifests) are product classes below, keyed from the
        # project configuration.
        for name in ("stories", "spec-product", "spec-arch", "spec-ux", "demo", "reviews"):
            assert CLASSES[name]["write"] == "tools", name

    def test_the_run_store_is_code_written(self):
        # Wall W3's substance: no caller may write step records by hand.
        for name in ("runs", "evidence", "local", "control", "decisions", "holdout-evaluations"):
            assert CLASSES[name]["write"] == "code", name

    def test_the_slice_state_class_globs_are_pinned(self):
        # The state classes the five slice rows grant or deny, pinned so a glob
        # change is a conscious change to the walls; the product classes are
        # pinned by their from_config keys above.
        assert CLASSES["brief"]["globs"] == [".factory/specs/product/brief.md"]
        assert CLASSES["deferred"]["globs"] == [".factory/deferred/**"]
        assert CLASSES["reviews"]["globs"] == [".factory/reviews/**"]
        assert CLASSES["stories"]["globs"] == [".factory/stories/**"]
        assert CLASSES["holdout"]["globs"] == [".factory/holdout/**"]
        assert CLASSES["runs"]["globs"] == [".factory/local/runs/**"]

    def test_a_person_edits_the_configuration(self):
        assert CLASSES["config"]["write"] == "none"

    def test_state_directory_globs_are_rooted_in_the_state_directory(self):
        for name in ("config", "control", "stories", "runs", "holdout", "holdout-evaluations"):
            for pattern in CLASSES[name]["globs"]:
                assert pattern.startswith(".factory/") or pattern == ".factory/factory.toml", name


def covering_classes(kind: str) -> list[str]:
    """The classes a `vsdd list <kind>` reveals: the class its glob falls in.

    The winner is the class whose glob is the longest directory prefix of the
    kind's glob. The [asset] glob spans the three spec classes
    (.factory/specs/*/assets/), which a prefix test cannot see; the walls
    table's comment names the exception, and the test carries it here.
    """
    glob = ARTIFACTS[kind]["glob"].split(",")[0].strip()
    hits: list[tuple[int, str]] = []
    for name, row in CLASSES.items():
        if name == "_product" or "globs" not in row:
            continue
        for pattern in row["globs"]:
            base = pattern.rstrip("*")
            if glob.startswith(base):
                hits.append((len(base), name))
    if not hits and kind == "asset":
        return ["spec-product", "spec-arch", "spec-ux"]
    longest = max(length for length, _ in hits)
    return sorted({name for length, name in hits if length == longest})


class TestRegistryVocabularyAgrees:
    def test_reveals_keys_use_the_artifact_kind_ids(self):
        # AU-T2-09: the [reveals] vocabulary is the registry's. "list scenario"
        # matched no kind; the kind ids are holdout-scenario, story, contract.
        kinds = {name for name, row in ARTIFACTS.items() if isinstance(row, dict)}
        for key in REVEALS:
            if key.startswith("list "):
                assert key[len("list "):] in kinds, key

    def test_reveals_name_defined_classes(self):
        for key, classes in REVEALS.items():
            unknown = set(classes) - CLASS_NAMES
            assert not unknown, f"{key}: {sorted(unknown)}"

    def test_one_reveal_entry_per_artifact_kind(self):
        # L-POLICY: every artifact kind has one `list <kind>` entry, so no kind
        # falls back to revealing every walled class.
        kinds = {name for name, row in ARTIFACTS.items() if isinstance(row, dict) and name != "_unknown"}
        listkeys = {key[len("list "):] for key in REVEALS if key.startswith("list ")}
        assert listkeys == kinds

    def test_each_reveal_entry_is_the_class_the_kind_s_glob_falls_in(self):
        for kind in {key[len("list "):] for key in REVEALS if key.startswith("list ")}:
            assert REVEALS[f"list {kind}"] == covering_classes(kind), kind
