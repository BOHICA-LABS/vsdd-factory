#!/usr/bin/env python3
"""Static checks for the vsdd-factory 1.0 rewrite.

Run from the repository root that `next/` stages:

    cd next && uv run python tools/check.py

The check list grows with the build: each package adds the checks for what it
ships (B1: manifest and MCP lint; B2: shell and launcher checks; B6: the full
lint suite over every shipped file). Every check is a function registered
below; tests/lint/ runs the same functions, so pytest and this script can
never disagree.

Standard library only; exits 0 when everything passes and 1 otherwise.
"""

from __future__ import annotations

import ast
import importlib.util
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
from pathlib import Path
from urllib.parse import urlparse

sys.dont_write_bytecode = True  # the plugin tree stays free of generated files

NEXT = Path(__file__).resolve().parent.parent
PLUGIN = NEXT / "plugins" / "vsdd-factory"
OLD_REPO_ROOT = NEXT.parent  # the checkout that stages next/; holds the canonical LICENSE
ROSTER_PATH = NEXT / "tests" / "lint" / "roster.toml"  # [platform] is where the floors live (B6)
SPEC_DIR = OLD_REPO_ROOT / "rewrite" / "spec"

REGISTRY: list = []


def check(fn):
    """Register a check function. Each prints nothing and raises AssertionError on failure."""
    REGISTRY.append(fn)
    return fn


class CheckError(Exception):
    pass


def expect(condition: bool, message: str) -> None:
    if not condition:
        raise CheckError(message)


def read_bytes(path: Path) -> bytes:
    expect(path.is_file(), f"missing file: {path.relative_to(NEXT)}")
    return path.read_bytes()


def read_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise CheckError(f"{path.relative_to(NEXT)} is not valid JSON: {exc}") from exc


def run(cmd: list[str], **kwargs):
    return subprocess.run(
        cmd, capture_output=True, text=True, timeout=60, check=False, **kwargs
    )


# ---------------------------------------------------------------------------
# B0: the staged repository skeleton


SKELETON_DIRS = (
    "workflow-lib",
    "tools",
    "docs",
    ".github/workflows",
    "tests/unit",
    "tests/hooks",
    "tests/lint",
    "tests/workflows",
    "tests/conformance",
    "tests/e2e",
    "plugins/vsdd-factory",
)

PLUGIN_DIRS = (
    "agents",
    "skills",
    "methods",
    "workflows",
    "schemas",
    "hooks",
    "templates",
    "observability",
    "assets/design-system",
    "assets/demo",
    "assets/scaffold",
    "py",
    "bin",
)


def assert_survives_fresh_clone(directory: Path, name: str) -> None:
    """Git does not track empty directories: a required directory must hold at least
    one file (a .gitkeep where nothing is shipped yet) or the layout check fails on
    every fresh clone, worktree and CI checkout."""
    expect(
        any(p.is_file() for p in directory.iterdir()),
        f"{name}/ holds no file; git does not track empty directories "
        "(add a .gitkeep so the directory survives a fresh clone)",
    )


@check
def skeleton_layout() -> None:
    for name in (
        "README.md",
        "CLAUDE.md",
        "CONTRIBUTING.md",
        "RELEASING.md",
        "CHANGELOG.md",
        "LICENSE",
        "pyproject.toml",
        "uv.lock",
        ".nvmrc",
        ".gitignore",
        ".semgrep.yml",
    ):
        expect((NEXT / name).is_file(), f"missing next/{name}")
    for rel in SKELETON_DIRS:
        expect((NEXT / rel).is_dir(), f"missing next/{rel}/")
        assert_survives_fresh_clone(NEXT / rel, f"next/{rel}")
    expect(
        re.fullmatch(r"\d+", (NEXT / ".nvmrc").read_text(encoding="utf-8").strip()) is not None,
        "next/.nvmrc must hold a bare Node major version",
    )
    pyproject = read_json_toml_section()
    expect(pyproject.get("package") is False, "[tool.uv] package = false is required")


def read_json_toml_section() -> dict:
    # Minimal read of [tool.uv] from pyproject.toml; tomllib is 3.11+, the floor.
    import tomllib

    data = tomllib.loads((NEXT / "pyproject.toml").read_text(encoding="utf-8"))
    return (data.get("tool") or {}).get("uv") or {}


# ---------------------------------------------------------------------------
# B1: manifest, plugin layout, MCP configuration, licence
#
# The manifest is compared against the whole design literal (packaging.md 2.4):
# "exactly the fields in 2.4" means a substituted or extra field is a defect,
# not a tolerated one.


MANIFEST_HOSTS = [
    "api.perplexity.ai",
    "mcp.tavily.com",
    "mcp.context7.com",
    "registry.npmjs.org",
    "github.com",
    "api.github.com",
    "*.githubusercontent.com",
    "*.docker.io",
    "*.docker.com",
]

MANIFEST_DESCRIPTION = (
    "Verified Spec-Driven Development for Claude Code: specifications, stories, test-first "
    "delivery, adversarial review, holdout evaluation and release gating, run by workflows "
    "and enforced by hooks."
)

USERCONFIG = {
    "perplexity_api_key": {
        "title": "Perplexity API key",
        "description": "Lets research agents search with Perplexity. Optional: leave empty to work without it.",
    },
    "tavily_api_key": {
        "title": "Tavily API key",
        "description": "Lets research agents search and extract with Tavily. Optional.",
    },
    "context7_api_key": {
        "title": "Context7 API key",
        "description": "Lets agents fetch current library documentation from Context7. Optional.",
    },
}

MANIFEST_OMITTED = (
    "$schema",
    "agents",
    "skills",
    "workflows",
    "hooks",
    "mcpServers",
    "commands",
    "dependencies",
    "defaultEnabled",
    # Deliberately omitted optional fields (packaging.md 2.4): the defaults are
    # right or the feature is unused.
    "privacyPolicyUrl",
    "termsOfServiceUrl",
    "metadata",
    "channels",
    "types",
    "lspServers",
    "outputStyles",
    "experimental",
)


@check
def plugin_manifest() -> None:
    path = PLUGIN / ".claude-plugin" / "plugin.json"
    manifest = read_json(path)
    # Exactly the top-level fields of the design literal, no more, no fewer.
    expect(
        sorted(manifest)
        == sorted(
            [
                "name",
                "displayName",
                "version",
                "description",
                "author",
                "homepage",
                "repository",
                "license",
                "keywords",
                "icon",
                "documentationUrl",
                "supportUrl",
                "engines",
                "network",
                "userConfig",
            ]
        ),
        f"plugin.json must hold exactly the fields of packaging.md 2.4, got {sorted(manifest)}",
    )
    expect(manifest["name"] == "vsdd-factory", "plugin.json name must be 'vsdd-factory'")
    expect(manifest["displayName"] == "VSDD Factory", "plugin.json displayName must be 'VSDD Factory'")
    expect(manifest["version"] == "1.0.0", "plugin.json version must be '1.0.0'")
    expect(
        manifest["description"] == MANIFEST_DESCRIPTION,
        "plugin.json description must be the design literal (packaging.md 2.4)",
    )
    author = manifest["author"]
    expect(
        sorted(author) == ["name", "url"],
        f"plugin.json author must hold exactly name and url, got {sorted(author)}",
    )
    expect(author.get("name") == "Bohica Labs", "plugin.json author.name must be 'Bohica Labs'")
    expect(author.get("url") == "https://github.com/bohica-labs", "plugin.json author.url mismatch")
    expect(
        manifest["homepage"] == "https://github.com/bohica-labs/vsdd-factory#readme",
        "plugin.json homepage mismatch",
    )
    expect(
        manifest["repository"] == "https://github.com/bohica-labs/vsdd-factory",
        "plugin.json repository mismatch",
    )
    expect(manifest["license"] == "MIT", "plugin.json license must be MIT")
    expect(
        manifest["keywords"]
        == ["vsdd", "spec-driven", "tdd", "sdlc", "workflows", "adversarial-review"],
        "plugin.json keywords must be the six from the design, in order",
    )
    expect(
        manifest["icon"] == "./.claude-plugin/icon.png",
        "plugin.json icon must be ./.claude-plugin/icon.png",
    )
    expect(
        manifest["engines"] == {"claude-code": f">={roster_platform()['min_claude_code']}"},
        "plugin.json engines must pin the Claude Code floor of tests/lint/roster.toml [platform]",
    )
    expect(
        manifest["documentationUrl"] == "https://github.com/bohica-labs/vsdd-factory/tree/main/docs",
        "plugin.json documentationUrl mismatch",
    )
    expect(
        manifest["supportUrl"] == "https://github.com/bohica-labs/vsdd-factory/issues",
        "plugin.json supportUrl mismatch",
    )
    expect(
        sorted(manifest["network"]) == ["hosts"],
        f"plugin.json network must hold exactly hosts, got {sorted(manifest['network'])}",
    )
    expect(
        sorted(manifest["network"]["hosts"]) == sorted(MANIFEST_HOSTS),
        "plugin.json network.hosts must be the nine design hosts",
    )
    user_config = manifest["userConfig"]
    expect(
        sorted(user_config) == ["context7_api_key", "perplexity_api_key", "tavily_api_key"],
        "plugin.json userConfig must hold exactly the three API keys",
    )
    for key, spec in user_config.items():
        expect(
            sorted(spec) == ["description", "sensitive", "title", "type"],
            f"userConfig.{key} must hold exactly type, sensitive, title and description",
        )
        expect(spec["sensitive"] is True, f"userConfig.{key} must be sensitive")
        expect(spec["type"] == "string", f"userConfig.{key} must be a string")
        expect(
            spec["title"] == USERCONFIG[key]["title"],
            f"userConfig.{key}.title must be {USERCONFIG[key]['title']!r}",
        )
        expect(
            spec["description"] == USERCONFIG[key]["description"],
            f"userConfig.{key}.description must be the design literal",
        )
    for omitted in MANIFEST_OMITTED:
        expect(omitted not in manifest, f"plugin.json must omit {omitted}")


@check
def plugin_layout() -> None:
    for rel in PLUGIN_DIRS:
        expect((PLUGIN / rel).is_dir(), f"missing plugin directory {rel}/")
        assert_survives_fresh_clone(PLUGIN / rel, f"plugins/vsdd-factory/{rel}")
    expect(
        not (PLUGIN / "package.json").exists(),
        "no package.json at the plugin root (an install-time package run is unwanted)",
    )
    expect(
        not (PLUGIN / "CLAUDE.md").exists(),
        "no CLAUDE.md at the plugin root (the platform does not load it; the validator warns)",
    )
    expect(
        not (PLUGIN / "settings.json").exists(),
        "no settings.json at the plugin root (the plugin ships no settings of its own)",
    )
    expect(
        (PLUGIN / "README.md").is_file(),
        "missing plugins/vsdd-factory/README.md (the platform links the plugin README)",
    )
    for rel in ("py/launch.sh", "py/find-root.sh", "py/hook.py", "py/cli.py", "bin/vsdd"):
        expect((PLUGIN / rel).is_file(), f"missing plugin file {rel}")
    expect(os.access(PLUGIN / "bin" / "vsdd", os.X_OK), "bin/vsdd must be executable")

    icon = PLUGIN / ".claude-plugin" / "icon.png"
    raw = read_bytes(icon)
    expect(raw[:8] == b"\x89PNG\r\n\x1a\n", "icon.png is not a PNG")
    expect(raw[12:16] == b"IHDR", "icon.png lacks an IHDR chunk")
    width = int.from_bytes(raw[16:20], "big")
    height = int.from_bytes(raw[20:24], "big")
    expect(width == height == 512, f"icon must be a square 512 px PNG, got {width}x{height}")


@check
def licence_parity() -> None:
    staged = (NEXT / "LICENSE").read_bytes()
    plugin_copy = (PLUGIN / "LICENSE").read_bytes()
    expect(staged == plugin_copy, "plugin LICENSE must be byte-identical to the root LICENSE (lint)")
    # Staging-only: the checkout that stages next/ holds the canonical LICENSE.
    # After promotion (CO2), NEXT is the repository root and the comparison above
    # is the whole check; OLD_REPO_ROOT then points outside the repository.
    old_root_licence = OLD_REPO_ROOT / "LICENSE"
    if OLD_REPO_ROOT != NEXT and old_root_licence.is_file():
        expect(old_root_licence.read_bytes() == plugin_copy, "staged LICENSE must match the repository LICENSE")
    expect(plugin_copy.lstrip().startswith(b"MIT License"), "the licence is MIT")


MCP_SERVER_KEYS = ["perplexity", "tavily", "context7", "playwright"]


@check
def mcp_configuration() -> None:
    path = PLUGIN / ".mcp.json"
    data = read_json(path)
    servers = data.get("mcpServers") or {}
    expect(sorted(servers) == sorted(MCP_SERVER_KEYS), f".mcp.json servers must be {MCP_SERVER_KEYS}")

    perplexity = servers["perplexity"]
    expect(perplexity.get("command") == "npx", "perplexity server runs through npx")
    pargs = perplexity.get("args") or []
    expect(
        len(pargs) == 2 and pargs[0] == "-y" and re.fullmatch(r"@perplexity-ai/mcp-server@\d+\.\d+\.\d+", pargs[1]),
        f"perplexity must pin an exact release (@perplexity-ai/mcp-server@<exact>), got {pargs}",
    )

    tavily = servers["tavily"]
    expect(tavily.get("type") == "http", "tavily is an http server")
    expect(tavily.get("url") == "https://mcp.tavily.com/mcp/", "tavily URL mismatch")
    expect(
        (tavily.get("headers") or {}).get("Authorization") == "Bearer ${user_config.tavily_api_key}",
        "tavily must take its bearer token from user configuration",
    )

    context7 = servers["context7"]
    expect(context7.get("type") == "http", "context7 is an http server")
    expect(context7.get("url") == "https://mcp.context7.com/mcp", "context7 URL mismatch")
    expect(
        (context7.get("headers") or {}).get("CONTEXT7_API_KEY")
        == "${user_config.context7_api_key}",
        "context7 must take its key from user configuration",
    )

    playwright = servers["playwright"]
    expect(playwright.get("command") == "npx", "playwright server runs through npx")
    wargs = playwright.get("args") or []
    expect(
        len(wargs) == 2 and wargs[0] == "-y" and re.fullmatch(r"@playwright/mcp@\d+\.\d+\.\d+", wargs[1]),
        f"playwright must pin an exact release (@playwright/mcp@<exact>), got {wargs}",
    )
    expect(
        "--allow-unrestricted-file-access" not in wargs,
        "playwright must not pass --allow-unrestricted-file-access: at the pinned version "
        "navigation to file:// URLs is blocked by default and that flag loosens it",
    )
    for arg in pargs + wargs:
        expect("@latest" not in arg, "a lint rule rejects @latest: pin exact versions")

    text = path.read_text(encoding="utf-8")
    manifest = read_json(PLUGIN / ".claude-plugin" / "plugin.json")
    refs = set(re.findall(r"\$\{user_config\.([a-z0-9_]+)\}", text))
    expect(refs == set(manifest.get("userConfig") or {}), ".mcp.json must substitute exactly the userConfig keys")

    # L-MCP: every remote host a server contacts is declared in network.hosts.
    hosts = (manifest.get("network") or {}).get("hosts") or []
    for name, server in servers.items():
        url = server.get("url")
        if server.get("type") == "http" and url:
            host = urlparse(url).netloc
            expect(
                host in hosts,
                f".mcp.json server {name!r} contacts {host!r}, which plugin.json network.hosts does not declare",
            )

    # No loosened file access through the environment either: the flag is banned
    # in args above, and the env equivalent must not smuggle it back in.
    for name, server in servers.items():
        env = server.get("env") or {}
        expect(
            "PLAYWRIGHT_MCP_ALLOW_UNRESTRICTED_FILE_ACCESS" not in env,
            f".mcp.json server {name!r} must not set PLAYWRIGHT_MCP_ALLOW_UNRESTRICTED_FILE_ACCESS "
            "in env: it loosens file access like the banned flag",
        )


# ---------------------------------------------------------------------------
# B2: shell sources and launcher behaviour


def shell_sources() -> list[Path]:
    return [PLUGIN / "py" / "launch.sh", PLUGIN / "py" / "find-root.sh", PLUGIN / "bin" / "vsdd"]


SYSPATH_INSERT = "sys.path.insert(0, str(Path(__file__).resolve().parent))"


@check
def entries_self_register_on_sys_path() -> None:
    """PYTHONSAFEPATH=1 keeps `uv run --script` from putting the entry's directory on
    sys.path, so a sibling `import vsdd` (B3 and later) only resolves when each entry
    inserts its own directory before any vsdd import (hooks.md 2.2, 2.4)."""
    for name in ("hook.py", "cli.py"):
        text = (PLUGIN / "py" / name).read_text(encoding="utf-8")
        at = text.find(SYSPATH_INSERT)
        expect(at != -1, f"{name} must carry its own sys.path insertion: {SYSPATH_INSERT}")
        head = text[:at]
        expect(
            "import vsdd" not in head and "from vsdd" not in head,
            f"{name} imports the vsdd package before inserting its directory on sys.path",
        )


@check
def shell_syntax() -> None:
    interpreters = [["sh"], ["bash", "--posix"]]
    if shutil.which("dash"):
        interpreters.append(["dash"])
    for script in shell_sources():
        text = script.read_text(encoding="utf-8")
        expect(text.startswith("#!/bin/sh\n"), f"{script.name} must be POSIX sh (#/bin/sh shebang)")
        for interp in interpreters:
            result = run(interp + ["-n", str(script)])
            expect(
                result.returncode == 0,
                f"{script.name} fails {interp[0]} -n: {result.stderr.strip()}",
            )


@check
def shell_lint() -> None:
    """L-SH: shellcheck passes on the launcher sources. It fails when shellcheck is
    absent: a missing linter must not silently pass the lint tier."""
    shellcheck = shutil.which("shellcheck")
    expect(
        shellcheck is not None,
        "shellcheck is not installed; L-SH requires it (brew install shellcheck)",
    )
    for script in shell_sources():
        result = run([shellcheck, "-s", "sh", "-x", "-P", "SCRIPTDIR", str(script)])
        expect(
            result.returncode == 0,
            f"shellcheck rejected {script.relative_to(NEXT)}:\n{(result.stdout + result.stderr).strip()}",
        )


GENERATED_SUFFIXES = {".pyc", ".pyo", ".swp", ".orig", ".rej"}


@check
def nothing_generated_under_plugin() -> None:
    """The plugin root changes with every version: no runtime or editor droppings
    travel with it (packaging.md 2.3). Bytecode goes to the user's cache via
    PYTHONPYCACHEPREFIX."""
    for path in sorted(PLUGIN.rglob("*")):
        rel = path.relative_to(NEXT)
        expect(path.name != "__pycache__", f"generated directory under the plugin: {rel}")
        expect(path.name != ".DS_Store", f"generated file under the plugin: {rel}")
        expect(path.suffix not in GENERATED_SUFFIXES, f"generated file under the plugin: {rel}")


@check
def shipped_files_are_not_gitignored() -> None:
    """A shipped file that an ignore rule matches vanishes from every fresh clone
    without warning: next/.gitignore becomes the repository root .gitignore at
    promotion, and the old root ignores an unanchored `.mcp.json`."""
    probe = run(["git", "-C", str(NEXT), "rev-parse", "--show-toplevel"])
    if probe.returncode != 0:
        return  # not a git checkout (a tarball, a staging copy); nothing to compare against
    files = sorted(p.relative_to(NEXT).as_posix() for p in PLUGIN.rglob("*") if p.is_file())
    expect(files, "the plugin directory holds no files")
    result = run(
        ["git", "-C", str(NEXT), "check-ignore", "--stdin"],
        input="\n".join(files) + "\n",
    )
    if result.returncode == 1:
        return  # exit 1: no path is ignored
    expect(result.returncode == 0, f"git check-ignore failed: {result.stderr.strip()}")
    ignored = [line for line in result.stdout.splitlines() if line.strip()]
    expect(
        not ignored,
        "shipped files match an ignore rule and would vanish on a fresh clone: "
        + ", ".join(ignored),
    )


@check
def shipped_python_is_offline_stdlib() -> None:
    banned = {"socket", "ssl", "http", "urllib", "ftplib", "smtplib", "telnetlib", "xmlrpc"}
    shipped = sorted((PLUGIN / "py").glob("*.py")) + sorted((PLUGIN / "py" / "vsdd").rglob("*.py"))
    for py in shipped:
        tree = ast.parse(py.read_text(encoding="utf-8"))
        names: set[str] = set()
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                names |= {alias.name.split(".")[0] for alias in node.names}
            elif isinstance(node, ast.ImportFrom):
                if node.level == 0 and node.module:
                    names.add(node.module.split(".")[0])
        for name in sorted(names):
            expect(
                name in sys.stdlib_module_names or name == "vsdd",
                f"{py.name} imports {name!r}: shipped Python is standard library only",
            )
            expect(name not in banned, f"{py.name} imports {name!r}: shipped Python never opens a network connection")


def launcher_env(base: Path, **overrides) -> dict:
    env = dict(os.environ)
    env.pop("VSDD_FACTORY_HOOKS", None)
    env["CLAUDE_PROJECT_DIR"] = str(base)
    env.update(overrides)
    return env


def launch(process_args: list[str], base: Path, stdin_text: str = "", **env_overrides):
    return run(
        ["sh", str(PLUGIN / "py" / "launch.sh")] + process_args,
        input=stdin_text,
        cwd=str(base),
        env=launcher_env(base, **env_overrides),
    )


def make_project(tmp: Path, factory: bool = True, legacy: bool = False) -> Path:
    base = tmp / ("factory-project" if factory else "plain-project")
    (base / ".factory").mkdir(parents=True, exist_ok=True)
    if factory:
        (base / ".factory" / "factory.toml").write_text("schema = 1\n", encoding="utf-8")
    if legacy:
        (base / ".factory" / "STATE.md").write_text("# pre-1.0 state\n", encoding="utf-8")
    return base


@check
def launcher_inert_outside_factory_projects() -> None:
    """The inert path: no factory project -> exit 0, silent, fast (design: about 8 ms)."""
    with tempfile.TemporaryDirectory(prefix="vsdd-check-") as td:
        base = make_project(Path(td), factory=False, legacy=False)
        started = time.perf_counter()
        result = launch(["hook", "gate"], base, stdin_text='{"tool_name":"Bash"}')
        elapsed_ms = (time.perf_counter() - started) * 1000
        expect(result.returncode == 0, f"inert gate must exit 0, got {result.returncode}: {result.stderr!r}")
        expect(result.stdout == "", f"inert gate must print nothing on stdout, got {result.stdout!r}")
        expect(result.stderr == "", f"inert gate must print nothing on stderr, got {result.stderr!r}")
        expect(elapsed_ms < 500, f"inert path took {elapsed_ms:.0f} ms; expected a few ms")
        # A session inside a linked worktree of an unrelated repository is inert too:
        # the worktree's .git file points at the main checkout, which has no factory state.
        worktree = Path(td) / "linked-worktree"
        worktree.mkdir()
        (worktree / ".git").write_text(
            f"gitdir: {td}/main-repo/.git/worktrees/story-x\n", encoding="utf-8"
        )
        (Path(td) / "main-repo" / ".git" / "worktrees" / "story-x").mkdir(parents=True)
        result = launch(["hook", "gate"], worktree, stdin_text="{}")
        expect(result.returncode == 0 and result.stdout == "" and result.stderr == "",
               "a linked worktree of a non-factory repository must stay inert")


@check
def launcher_blocks_without_uv() -> None:
    """uv missing -> exit 2, message names the fix (the loud-failure contract)."""
    with tempfile.TemporaryDirectory(prefix="vsdd-check-") as td:
        base = make_project(Path(td))
        # gate: exit 2 with the fault and the fix on stderr.
        result = launch(
            ["hook", "gate"], base, stdin_text="{}", PATH="/usr/bin:/bin", VSDD_FACTORY_SELFTEST_NO_UV="1"
        )
        expect(result.returncode == 2, f"uv-missing gate must exit 2, got {result.returncode}")
        expect("uv is not installed" in result.stderr, f"gate message must name the fault: {result.stderr!r}")
        expect("brew install uv" in result.stderr, "gate message must name the fix")
        # brief: exit 0 with the fixed down briefing, person and model both told.
        result = launch(
            ["hook", "brief"], base, stdin_text="{}", PATH="/usr/bin:/bin", VSDD_FACTORY_SELFTEST_NO_UV="1"
        )
        expect(result.returncode == 0, f"uv-missing brief must exit 0, got {result.returncode}")
        briefing = json.loads(result.stdout)
        expect("systemMessage" in briefing, "the down briefing must carry a systemMessage")
        expect("gates are down" in briefing["hookSpecificOutput"]["additionalContext"],
               "the down briefing must tell the model the gates are off")
        # notify: silent exit 0.
        result = launch(
            ["hook", "notify"], base, stdin_text="{}", PATH="/usr/bin:/bin", VSDD_FACTORY_SELFTEST_NO_UV="1"
        )
        expect(result.returncode == 0, f"uv-missing notify must exit 0, got {result.returncode}")


@check
def launcher_modes_on_a_live_project() -> None:
    """With uv present: gate allows through the stub runner, cli answers --version."""
    uv = shutil.which("uv")
    if uv is None:
        for candidate in ("/opt/homebrew/bin/uv", "/usr/local/bin/uv"):
            if Path(candidate).is_file():
                uv = candidate
                break
    if uv is None:
        raise CheckError("uv is not installed; install it to run the launcher checks")

    with tempfile.TemporaryDirectory(prefix="vsdd-check-") as td:
        base = make_project(Path(td))
        # A full PreToolUse payload (HD11): the runner reads hook_event_name,
        # and a payload without it is bad input that blocks.
        gate_payload = json.dumps(
            {
                "hook_event_name": "PreToolUse",
                "tool_name": "Bash",
                "tool_input": {"command": "echo ok"},
                "session_id": "check-session",
                "cwd": str(base),
            }
        )
        result = launch(["hook", "gate"], base, stdin_text=gate_payload)
        expect(result.returncode == 0, f"gate through the runner must exit 0, got {result.returncode}: {result.stderr!r}")

        result = launch(["hook", "brief"], base, stdin_text="{}")
        expect(result.returncode == 0, "brief on a live project must exit 0")

        # The emergency switch silences every mode; brief explains it.
        result = launch(["hook", "gate"], base, stdin_text="{}", VSDD_FACTORY_HOOKS="off")
        expect(result.returncode == 0 and result.stdout == "", "VSDD_FACTORY_HOOKS=off must silence gate")
        result = launch(["hook", "brief"], base, stdin_text="{}", VSDD_FACTORY_HOOKS="off")
        expect(result.returncode == 0 and "VSDD_FACTORY_HOOKS=off" in result.stdout,
               "brief must announce the emergency switch")

        # cli entry: --version through the launcher and through bin/vsdd.
        result = launch(["cli", "--version"], base)
        expect(result.returncode == 0, f"cli --version must exit 0, got {result.returncode}: {result.stderr!r}")
        expect(result.stdout.strip() == "vsdd 1.0.0", f"unexpected --version output: {result.stdout!r}")

        result = run([str(PLUGIN / "bin" / "vsdd"), "--version"], cwd=str(base))
        expect(result.returncode == 0, f"bin/vsdd --version must exit 0, got {result.returncode}: {result.stderr!r}")
        expect(result.stdout.strip() == "vsdd 1.0.0", f"unexpected bin/vsdd output: {result.stdout!r}")

        result = run([str(PLUGIN / "bin" / "vsdd", )], cwd=str(base))
        expect(result.returncode == 2, f"bin/vsdd with no subcommand must exit 2, got {result.returncode}")

        # Legacy project: inert everywhere except the session-start briefing.
        legacy_base = make_project(Path(td), factory=False, legacy=True)
        result = launch(["hook", "gate"], legacy_base, stdin_text="{}")
        expect(result.returncode == 0 and result.stdout == "", "legacy gate stays inert")
        result = launch(["hook", "brief"], legacy_base, stdin_text="{}")
        expect(result.returncode == 0, "legacy brief must exit 0")
        expect("pre-1.0" in result.stdout, "legacy brief must print the one migration line")


# ---------------------------------------------------------------------------
# B4a: the workflow prelude, its stamped copies, and the generated agent texts
#
# The build (tools/build.py) regenerates the marked sub-blocks inside
# workflow-lib/prelude.js from the roster and the schemas, stamps the prelude
# into every workflow, and stamps the shared block and the severity table into
# every agent file. These checks hold the committed tree to all of that
# (packaging.md 2.5; blueprint 4.14) without duplicating build.py's logic.

B4A_ROSTER_AGENTS = (
    "product-owner", "architect", "ux-designer", "story-writer", "codebase-analyzer",
    "researcher", "spike-runner", "test-writer", "implementer", "verifier",
    "demo-recorder", "technical-writer", "devops", "adversary", "code-reviewer",
    "security-reviewer", "holdout-evaluator", "ui-reviewer", "scenario-reviewer", "clerk",
)

PRELUDE_TABLE_MARKERS = ("// >>> generated:tables", "// <<< generated:tables")
PRELUDE_SCHEMA_MARKERS = ("// >>> generated:schemas", "// <<< generated:schemas")
PRELUDE_STAMP_MARKERS = ("// >>> stamped:prelude", "// <<< stamped:prelude")


def _strip_region(text: str, markers: tuple[str, str]) -> str:
    start = text.find(markers[0])
    end = text.find(markers[1])
    if start < 0 or end < 0:
        return text
    # Trim the seam newlines: the region is replaced wholesale, so the count
    # must measure the prelude's own lines, not the splice offsets.
    return text[:start].rstrip("\n") + "\n\n" + text[end + len(markers[1]):].lstrip("\n")


@check
def roster_parses_and_matches_the_blueprint() -> None:
    """tests/lint/roster.toml: four tiers, the 20 agents, and the two roster overrides."""
    roster = tomllib.loads((NEXT / "tests" / "lint" / "roster.toml").read_text())
    expect(set(roster["tiers"]) == {"planner", "builder", "reviewer", "clerk"}, "roster must have the four tiers")
    expect(roster["tiers"]["clerk"] == {"model": "sonnet", "effort": "low", "color": "yellow"}, "the clerk tier is sonnet at low effort, yellow")
    expect(roster["tiers"]["reviewer"] == {"model": "opus", "effort": "xhigh", "color": "red"}, "the reviewer tier is opus at xhigh, red")
    expect(roster["tiers"]["planner"] == {"model": "fable", "effort": "xhigh", "color": "blue"}, "the planner tier is fable at xhigh, blue")
    expect(roster["tiers"]["builder"] == {"model": "sonnet", "effort": "xhigh", "color": "green"}, "the builder tier is sonnet at xhigh, green")
    expect(set(roster["agents"]) == set(B4A_ROSTER_AGENTS), "roster agents must be the 20 of blueprint 4.3")
    expect(
        roster["overrides"] == {"ingest/map": "builder", "ingest/analyze": "builder"},
        "the roster's [overrides] carries exactly the two ingest inventory stages (D5)",
    )


@check
def build_is_current() -> None:
    """tools/build.py --check: no stamped or generated copy has drifted."""
    spec = importlib.util.spec_from_file_location("vsdd_build", NEXT / "tools" / "build.py")
    build = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(build)
    drifted, problems = build.build(check_only=True)
    expect(not problems, "; ".join(problems[:5]))
    expect(not drifted, f"{len(drifted)} file(s) drifted; run tools/build.py: {', '.join(drifted[:5])}")


@check
def prelude_respects_platform_constraints() -> bool:
    """Declarations only; no clocks; at most 400 non-generated lines (packaging.md 2.5)."""
    text = (NEXT / "workflow-lib" / "prelude.js").read_text()
    body = _strip_region(_strip_region(text, PRELUDE_TABLE_MARKERS), PRELUDE_SCHEMA_MARKERS)
    # Drop whole-line comments before searching: the header comment legitimately names the banned calls.
    code = re.sub(r"^\s*//.*$", "", body, flags=re.M)
    for pattern in (r"^import\s", r"^export\s", r"\bDate\.now\(", r"\bMath\.random\(", r"new Date\(\)"):
        expect(re.search(pattern, code, re.M) is None, f"prelude must not contain {pattern}")
    lines = len(body.splitlines())
    expect(lines <= 400, f"prelude exceeds the 400-line cap ({lines} lines)")
    expect(text.count(PRELUDE_STAMP_MARKERS[0]) == 0, "the prelude source must not carry the stamped marker")


@check
def workflow_scripts_have_the_layout_rule_shape() -> None:
    """Line 1 opens meta, a lone } closes it, and exactly one marker pair follows."""
    directories = [PLUGIN / "workflows", NEXT / "tests" / "workflows" / "toys"]
    for directory in directories:
        for path in sorted(directory.glob("*.js")):
            lines = path.read_text().splitlines()
            expect(lines and lines[0] == "export const meta = {", f"{path}: line 1 must open the meta literal")
            closes = [i for i, line in enumerate(lines) if line == "}"]
            expect(closes, f"{path}: the meta object never closes on a lone }} line")
            rest = [line for line in lines[closes[0] + 1:] if line.strip()]
            expect(
                bool(rest) and rest[0].startswith(PRELUDE_STAMP_MARKERS[0]),
                f"{path}: the stamped prelude marker must follow the meta object",
            )
            text = path.read_text()
            expect(text.count(PRELUDE_STAMP_MARKERS[0]) == 1 and text.count(PRELUDE_STAMP_MARKERS[1]) == 1,
                   f"{path}: exactly one stamped marker pair expected")


@check
def agent_texts_are_stamped_variants_of_one_source() -> None:
    """agent-shared.md and severity.md exist, carry the marker contract, and prelude imports nothing."""
    shared = NEXT / "workflow-lib" / "agent-shared.md"
    severity = NEXT / "workflow-lib" / "severity.md"
    expect(shared.is_file() and severity.is_file(), "workflow-lib must ship agent-shared.md and severity.md")
    expect("## Ground rules" in shared.read_text(), "the shared block must open its ground rules")
    for row in ("critical", "high", "medium", "low"):
        expect(row in severity.read_text(), f"the severity table is missing the {row} row")


# ---------------------------------------------------------------------------
# B3: the shared core package, and C1: the result schemas
#
# Real behaviour lives in tests/unit/; these checks cover what the shipped tree
# must look like even before anyone runs pytest.

CORE_MODULES = ("project.py", "config.py", "frontmatter.py", "events.py", "schema.py")


@check
def shared_core_modules_present() -> None:
    pkg = PLUGIN / "py" / "vsdd"
    expect((pkg / "__init__.py").is_file(), "missing py/vsdd/__init__.py")
    for name in CORE_MODULES:
        expect((pkg / name).is_file(), f"missing py/vsdd/{name}")


def roster_platform() -> dict:
    """The roster's [platform] table: the floors, stated once (B6)."""
    return tomllib.loads(ROSTER_PATH.read_text())["platform"]


@check
def python_compiles_on_the_floor() -> None:
    """L-PY: the shipped package parses on the Python floor of
    tests/lint/roster.toml [platform] (packaging.md, assigned to B3). ast
    feature_version refuses syntax newer than the floor, so a 3.12-only
    construct cannot reach a user whose uv resolved 3.11."""
    floor_text = roster_platform()["python_floor"]
    floor = tuple(int(part) for part in floor_text.split("."))
    for py in sorted((PLUGIN / "py").rglob("*.py")):
        source = py.read_text(encoding="utf-8")
        try:
            ast.parse(source, filename=str(py), feature_version=floor)
        except SyntaxError as exc:
            raise CheckError(
                f"{py.relative_to(NEXT)} does not parse on Python "
                f"{floor[0]}.{floor[1]}: {exc.msg} (line {exc.lineno})"
            ) from exc


@check
def schemas_are_the_single_source() -> None:
    """Every shipped schema parses, is draft 2020-12, and its $id matches the
    file name; every $ref resolves to a sibling schema file; and the set is
    exactly the result schemas of blueprint 4.10 plus the event log line."""
    schemas = sorted((PLUGIN / "schemas").glob("*.schema.json"))
    expect(schemas, "no schemas shipped")
    required = {
        "finding.schema.json",
        "gate-result.schema.json",
        "pending-output.schema.json",
        "caller-marker.schema.json",
        "review-data.schema.json",
        "claim.schema.json",
        "fix-data.schema.json",
        "ruling-data.schema.json",
        "holdout-data.schema.json",
        "intake-data.schema.json",
        "implement-data.schema.json",
        "story-unit.schema.json",
        "story-draft-data.schema.json",
        "spike-request.schema.json",
        "spike-finding.schema.json",
        "step-result.schema.json",
        "verdict.schema.json",
        "workflow-result.schema.json",
        "survey.schema.json",
        "checkpoint-result.schema.json",
        "delivery-data.schema.json",
        "wave-gate-data.schema.json",
        "event.schema.json",
        "run-record.schema.json",
        "step-record.schema.json",
        "evidence-record.schema.json",
        "closure-record.schema.json",
        "cycle-record.schema.json",
        "decision-record.schema.json",
        "lock.schema.json",
    }
    shipped = {p.name for p in schemas}
    expect(shipped == required, f"schemas must be exactly the result-schema set; missing: {sorted(required - shipped)}, extra: {sorted(shipped - required)}")
    names: set[str] = set()
    texts: dict[str, str] = {}
    for path in schemas:
        schema = read_json(path)
        expect(
            schema.get("$schema") == "https://json-schema.org/draft/2020-12/schema",
            f"{path.name} must declare the draft 2020-12 meta-schema",
        )
        expect(schema.get("$id") == path.name, f"{path.name} must carry its file name as $id")
        expect(isinstance(schema.get("type"), str), f"{path.name} must type its root")
        names.add(path.name)
        texts[path.name] = json.dumps(schema)
    for name, text in texts.items():
        start = 0
        while True:
            at = text.find('"$ref": "', start)
            if at == -1:
                break
            end = text.find('"', at + 9)
            target = text[at + 9 : end]
            expect(target in names, f"{name}: $ref {target!r} does not resolve to a shipped schema")
            start = end


@check
def schema_fixtures_exist_and_round_trip() -> None:
    """Every schema has valid and invalid fixtures, and the shipped checker
    agrees with both sides (C1's guarantee, exercised without pytest)."""
    sys.dont_write_bytecode = True  # the plugin tree must stay free of __pycache__
    sys.path.insert(0, str(PLUGIN / "py"))
    from vsdd.schema import load_registry, validate  # noqa: E402

    fixtures = NEXT / "tests" / "unit" / "fixtures" / "schemas"
    registry = load_registry(PLUGIN / "schemas")
    expect(bool(registry), "the schema registry is empty")
    for name in sorted(registry):
        stem = name[: -len(".schema.json")]
        directory = fixtures / stem
        expect(directory.is_dir(), f"no fixture directory for {name}")
        valid = sorted(directory.glob("valid_*.json"))
        invalid = sorted(directory.glob("invalid_*.json"))
        expect(valid, f"no valid fixture for {name}")
        expect(invalid, f"no invalid fixture for {name}")
        for path in valid:
            instance = read_json(path)
            errors = validate(instance, registry[name], registry)
            expect(not errors, f"valid fixture {path.name} fails {name}: {errors}")
        for path in invalid:
            instance = read_json(path)
            errors = validate(instance, registry[name], registry)
            expect(errors, f"invalid fixture {path.name} unexpectedly passes {name}")
            sidecar = path.parent / (path.stem + ".expect.txt")
            expect(sidecar.is_file(), f"invalid fixture {path.name} lacks its {sidecar.name} sidecar")
            expected = sidecar.read_text(encoding="utf-8").strip()
            rendered = "\n".join(str(e) for e in errors)
            expect(expected in rendered, f"{sidecar.name}: {expected!r} not in {rendered!r}")


@check
def frontmatter_reader_smoke() -> None:
    """The shipped reader parses a representative flat block and refuses a
    construct outside the subset (hooks.md 2.4)."""
    sys.dont_write_bytecode = True  # the plugin tree must stay free of __pycache__
    sys.path.insert(0, str(PLUGIN / "py"))
    from vsdd.frontmatter import FrontmatterError, parse  # noqa: E402

    meta = parse("---\nid: S-9.01\nstatus: draft\ncontracts: [BC-9.02.001]\n---\nbody")
    expect(
        meta == {"id": "S-9.01", "status": "draft", "contracts": ["BC-9.02.001"]},
        "the frontmatter reader misparsed a flat block",
    )
    try:
        parse("---\nratio: 1.5\n---\n")
    except FrontmatterError:
        pass
    else:
        raise CheckError("frontmatter reader accepted a float outside the subset")


@check
def relayed_outputs_require_every_property() -> None:
    """A command a script relays prints every field of its output schema on
    every exit (topics/result-schemas.md), so set(required) == set(properties)
    for each relayed output schema: a missing required field is a field the
    script reads into the void (CATCHUP-R3)."""
    relayed = (
        "survey",
        "checkpoint-result",
        "delivery-data",
        "story-unit",
        "wave-gate-data",
        "pending-output",
    )
    for name in relayed:
        schema = read_json(PLUGIN / "schemas" / f"{name}.schema.json")
        properties = set(schema.get("properties") or {})
        required = set(schema.get("required") or [])
        expect(
            required == properties,
            f"{name}.schema.json: the relay rule needs required == properties; "
            f"missing from required: {sorted(properties - required)}, not a property: {sorted(required - properties)}",
        )


def _without_descriptions(node):
    """A copy of a schema node with every description annotation dropped."""
    if isinstance(node, list):
        return [_without_descriptions(x) for x in node]
    if not isinstance(node, dict):
        return node
    return {k: _without_descriptions(v) for k, v in node.items() if k != "description"}


def _property(schema_name: str, prop: str):
    schema = read_json(PLUGIN / "schemas" / f"{schema_name}.schema.json")
    return _without_descriptions((schema.get("properties") or {}).get(prop))


@check
def composed_schemas_match_their_sources() -> None:
    """The compositions the old prelude built from $ref are copied by hand
    between the shipped files (CATCHUP-R9); each copied property must equal its
    source property, descriptions aside, so one side cannot change alone."""
    for prop in ("status", "progress", "decisions", "next"):
        expect(
            _property("checkpoint-result", prop) == _property("workflow-result", prop),
            f"checkpoint-result.{prop} has drifted from workflow-result.{prop}",
        )
    expect(
        _property("survey", "next") == _property("workflow-result", "next"),
        "survey.next has drifted from workflow-result.next",
    )
    gate_props = read_json(PLUGIN / "schemas" / "gate-result.schema.json").get("properties") or {}
    for prop in gate_props:
        expect(
            _property("gate-result", prop) == _property("wave-gate-data", prop),
            f"wave-gate-data.{prop} has drifted from gate-result.{prop}",
        )


@check
def spec_literals_are_the_generated_set() -> None:
    """Every S_* identifier in a spec code block or a workflow body is a literal
    the build generates from <plugin>/schemas/*.schema.json (CATCHUP-R1): a
    ported script that names a deleted composite must fail here, not with a
    ReferenceError on the platform."""
    spec = importlib.util.spec_from_file_location("vsdd_build_check", NEXT / "tools" / "build.py")
    build = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(build)
    generated = set(build.LITERAL_NAMES.values())
    token_re = re.compile(r"(?<![A-Za-z0-9_])(S_[A-Z0-9_]+)(?![A-Za-z0-9_])")
    fence_re = re.compile(r"^```", re.M)

    def fenced_code_blocks(text: str):
        parts = fence_re.split(text)
        for part in parts[1::2]:  # odd slices are inside fences
            yield part

    offenders: dict[str, set] = {}

    spec_dir = NEXT.parent / "rewrite" / "spec"
    spec_topics = sorted(spec_dir.rglob("*.md")) if spec_dir.is_dir() else []
    for path in spec_topics:
        text = path.read_text(encoding="utf-8")
        found = {t for block in fenced_code_blocks(text) for t in token_re.findall(block)}
        unknown = found - generated
        if unknown:
            offenders.setdefault(str(path), set()).update(unknown)

    for path in build._workflow_files():
        text = path.read_text()
        start, end = text.find("// >>> stamped:prelude"), text.find("// <<< stamped:prelude")
        body = text[:start] + text[end + len("// <<< stamped:prelude"):] if start >= 0 and end >= 0 else text
        unknown = set(token_re.findall(body)) - generated
        if unknown:
            offenders.setdefault(str(path), set()).update(unknown)

    expect(
        not offenders,
        "S_* names that are not generated literals: "
        + "; ".join(f"{p}: {sorted(n)}" for p, n in sorted(offenders.items())),
    )


CHECKPOINT_CALL_RE = re.compile(r"(vsdd checkpoint[^\n`]*)")
FENCE_SPLIT_RE = re.compile(r"^```", re.M)


def _checkpoint_problems(call: str, in_fence: bool) -> list[str]:
    """One invocation against commands/checkpoint.md's syntax,
    `vsdd checkpoint <workflow> --run <id> [--outcome failed]` (--json last). The
    script texts' conditional, `${failed ? ' --outcome failed' : ''}`, is resolved
    first; any other interpolation is a value placeholder."""
    call = re.sub(r"\$\{failed[^}]*' --outcome failed'[^}]*\}", " --outcome failed", call)
    call = re.sub(r"\$\{[^}]*\}", "V", call)
    tokens = call.replace("[", " ").replace("]", " ").split()
    rest = tokens[2:] if tokens[:2] == ["vsdd", "checkpoint"] else []
    if not rest:
        return []  # the command named with nothing invoked
    if rest[0] == "brief":
        return []  # the brief variant: [[command:checkpoint-brief]], no --run
    if not in_fence and not any(token.startswith("--") for token in rest):
        return []  # a prose mention ("vsdd checkpoint releases it"), no invocation to grammar
    problems: list[str] = []
    i = 1
    while i < len(rest):
        token = rest[i]
        if token == "--run":
            if i + 1 >= len(rest):
                problems.append("--run carries no run id")
                break
            i += 2
            continue
        if token == "--outcome":
            value = rest[i + 1] if i + 1 < len(rest) else ""
            if value != "failed":
                problems.append(f"--outcome must be failed, got {value!r} (commands/checkpoint.md)")
            i += 2
            continue
        if token == "--json":
            break  # the command ends here; the rest of the line is the script's JS
        if token.startswith("--"):
            problems.append(
                f"flag {token!r} is not in the syntax `vsdd checkpoint <workflow> --run <id> "
                "[--outcome failed]`"
            )
            i += 1
            continue
        problems.append(f"unexpected argument {token!r}")
        i += 1
    return problems


@check
def checkpoint_calls_match_the_signature() -> None:
    """Every `vsdd checkpoint` invocation in a spec file or a toy body matches
    commands/checkpoint.md (P3SEAM-10, P3COV-10, CATCHUP-R10): a `--subject` a
    parser would refuse, or an `--outcome` other than failed, stalls the closing
    command and holds the run's lock. Inside a fenced code block or a toy body
    every call is grammar-checked; in prose only a candidate that carries flags is
    one, so a mention of the command is never mistaken for an invocation."""
    sources: list[tuple[str, str, bool]] = []
    spec_dir = NEXT.parent / "rewrite" / "spec"
    if spec_dir.is_dir():
        for path in sorted(spec_dir.rglob("*.md")):
            sources.append((f"rewrite/spec/{path.relative_to(spec_dir)}", path.read_text(encoding="utf-8"), True))
    for path in sorted((NEXT / "tests" / "workflows" / "toys").glob("*.js")):
        sources.append((f"next/tests/workflows/toys/{path.name}", path.read_text(), False))
    problems: list[str] = []
    for label, text, is_markdown in sources:
        if is_markdown:
            # Fence-split: the odd slices are inside ``` pairs and are code.
            slices = FENCE_SPLIT_RE.split(text)
            segments = [(segment, index % 2 == 1) for index, segment in enumerate(slices)]
        else:
            segments = [(text, True)]
        for segment, fenced in segments:
            for match in CHECKPOINT_CALL_RE.finditer(segment):
                for problem in _checkpoint_problems(match.group(1), fenced):
                    problems.append(f"{label}: {problem} (in {match.group(1)!r})")
    expect(not problems, "; ".join(problems[:10]))


PRODUCT_CLASS_KEYS = ["tests", "harness", "ci", "repo-config", "manifests", "docs", "source"]


@check
def policy_tables_are_consistent() -> None:
    """The three shipped policy tables (C2) parse and cross-reference: a wall
    row may only name classes, vsdd groups, and command families the design's
    command-families table defines, every row denies the runs class, the
    person-only and governed constants are the single copies in the design's
    tables/callers.toml, and the reveals vocabulary is the artifact registry's."""
    policy_dir = PLUGIN / "py" / "vsdd" / "policy"
    artifacts = tomllib.loads((policy_dir / "artifacts.toml").read_text())
    classes = tomllib.loads((policy_dir / "classes.toml").read_text())
    walls = tomllib.loads((policy_dir / "walls.toml").read_text())

    computed = {"own-worktree", "project", "tmp", "outside"}
    class_names = {name for name in classes if name != "_product"} | computed
    groups = walls.get("callers", {})
    rows = {
        name: row
        for name, row in walls.items()
        if isinstance(row, dict) and name not in ("callers", "reveals")
    }
    expect("main" in rows and "other" in rows, "walls.toml must carry the main and other rows")
    kinds = {name for name, row in artifacts.items() if isinstance(row, dict)}

    for name, row in rows.items():
        for key in ("read_deny", "read_allow", "write"):
            unknown = {str(grant).split(":")[0] for grant in row.get(key, [])} - class_names
            expect(
                not unknown,
                f"walls.toml [{name}] {key} names undefined path classes: {sorted(unknown)}",
            )
        unknown_groups = set(row.get("vsdd", [])) - set(groups)
        expect(
            not unknown_groups,
            f"walls.toml [{name}] holds undefined vsdd groups: {sorted(unknown_groups)}",
        )
        expect(
            "runs" in row.get("read_deny", []),
            f"walls.toml [{name}] does not deny the runs class (wall W3, every row)",
        )

    for name, row in rows.items():
        expect(
            "relaxed_by" not in row or name == "main",
            f"walls.toml [{name}] carries relaxed_by; only the main row may (decision D6)",
        )
        if name.startswith("vsdd-factory:") and name not in (
            "vsdd-factory:spike-runner", "vsdd-factory:clerk"
        ):
            expect(
                row.get("dispatch") == ["vsdd-factory:spike-runner"],
                f"walls.toml [{name}] must dispatch the spike runner and nothing else",
            )
    command_families = set(
        tomllib.loads((SPEC_DIR / "tables" / "command-families.toml").read_text())["families"]
    ) | {"*"}
    for name, row in rows.items():
        unknown = set(row.get("commands", [])) - command_families
        expect(
            not unknown,
            f"walls.toml [{name}] commands name undefined command families: {sorted(unknown)} "
            "(tables/command-families.toml is the one list of family names)",
        )

    measure_holders = {n for n, r in rows.items() if "measure" in r.get("vsdd", [])}
    expect(
        measure_holders == {"vsdd-factory:clerk"},
        f"only the clerk's row holds the measure group: {sorted(measure_holders)}",
    )
    callers_spec = tomllib.loads((SPEC_DIR / "tables" / "callers.toml").read_text())
    human_only = set(callers_spec["HUMAN_ONLY"])
    governed = set(callers_spec["GOVERNED"])
    granted_subcommands = {sub for members in groups.values() for sub in members}
    expect(
        not human_only & granted_subcommands,
        "a [callers] group carries a person-only subcommand: "
        + ", ".join(sorted(human_only & granted_subcommands)),
    )
    for gov in sorted(governed):
        expect(
            any(entry == gov or gov.startswith(entry) for entry in granted_subcommands),
            f"the governed subcommand {gov!r} matches no granted group entry, so merge-guard's "
            "route list would bite on nothing",
        )
    expect(not governed & human_only, "a subcommand is both person-only and governed")

    for key, revealed in walls.get("reveals", {}).items():
        if key.startswith("list "):
            kind = key[len("list "):]
            expect(
                kind in kinds,
                f"reveals key {key!r} matches no artifact kind in artifacts.toml",
            )
        unknown = set(revealed) - class_names
        expect(not unknown, f"reveals key {key!r} names undefined classes: {sorted(unknown)}")

    for name in PRODUCT_CLASS_KEYS:
        row = classes.get(name)
        expect(isinstance(row, dict), f"classes.toml is missing the {name} class")
        expect(
            str(row.get("from_config", "")).startswith("paths."),
            f"classes.toml [{name}] must take its globs from the project configuration",
        )
    precedence = classes.get("_product", {}).get("precedence", [])
    expect(
        precedence == PRODUCT_CLASS_KEYS,
        f"classes.toml [_product] precedence must be {PRODUCT_CLASS_KEYS}, got {precedence}",
    )
    expect(
        precedence.index("tests") < precedence.index("source"),
        "the precedence must resolve tests before source, or src/**/tests.rs is source",
    )


@check
def event_schema_is_the_shipped_file() -> None:
    """The event log's schema is the shipped JSON file, loaded by the writer
    (CATCHUP-R15): the file is the single source, the log's line admits what the
    launcher writes, and the documented type list lives in its table."""
    sys.dont_write_bytecode = True
    sys.path.insert(0, str(PLUGIN / "py"))
    from vsdd.events import EVENT_SCHEMA  # noqa: E402

    shipped = read_json(PLUGIN / "schemas" / "event.schema.json")
    expect(
        EVENT_SCHEMA == shipped,
        "py/vsdd/events.py must load its schema from plugins/vsdd-factory/schemas/event.schema.json "
        "(an inline copy has drifted from the shipped file)",
    )
    expect(
        shipped.get("properties", {}).get("session_id", {}).get("type") == ["string", "null"],
        "the event schema must admit the launcher's null session_id",
    )


# ---------------------------------------------------------------------------

def main() -> int:
    failures: list[tuple[str, str]] = []
    for fn in REGISTRY:
        name = fn.__name__
        try:
            fn()
        except CheckError as exc:
            failures.append((name, str(exc)))
            print(f"FAIL {name}: {exc}")
        except AssertionError as exc:
            failures.append((name, str(exc)))
            print(f"FAIL {name}: {exc}")
        except Exception as exc:  # noqa: BLE001 - a check crash is a failure, not a traceback for the reader
            failures.append((name, f"{type(exc).__name__}: {exc}"))
            print(f"FAIL {name}: {type(exc).__name__}: {exc}")
        else:
            print(f"PASS {name}")
    print(f"{len(REGISTRY) - len(failures)}/{len(REGISTRY)} checks passed"
          + ("" if not failures else f" ({len(failures)} failed)"))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
