"""Package B6 lint: the manifest, agent files, workflow files and the roster.

The slice checks (packaging/build-plan.md "The thin slice"; table:lint-checks):
L-MAN, L-AGENT, L-ROSTER, L-KIND, the workflow-call half of L-XREF, the
per-file half of L-BUDGET and, over the files this lint reads, the reference
half of L-XREF (every ``vsdd-factory:<name>`` resolves). Everything here is
held to ``tests/lint/roster.toml``, the roster as data. While packages are
still being built only "nothing on disk is absent from the roster" is
enforced; the roster lists the whole design, so the checks activate as agent
and workflow files land. The reverse half of L-ROSTER — nothing in the roster
is absent from disk — is implemented and skipped until E1 part 2.

Division of labour with tools/check.py (one fact, one place): the manifest's
field set and literals, README and LICENSE existence and parity, and the absence
of plugin-root package.json/CLAUDE.md/settings.json are check.py's; the engines
floor against the roster's [platform], icon existence, agent files, workflow
call edges, the tier-override rule and the budgets are ours.

The stamped-region comparisons delegate to tools/build.py itself, so the shared
block, the severity table and the finding-key rule have one source and no
hand-written copy to drift (agents-skills.md 2.7); reviewer status is
``build_mod.is_reviewer``, the build's own decision.

MCP tool names: an agent lists a bundled server's tool in both spellings
(``mcp__plugin_vsdd-factory_<server>__<tool>`` and ``mcp__<server>__<tool>``;
topics/research-and-browser-tools.md), a name a spelling of which resolves is
accepted, and a bare MCP tool name is rejected, because a bare name resolves to
nothing and the platform drops it silently (topics/platform-proven.md, item 10).

Deferred from this slice, each with the package that widens it: L-METHOD (needs
the shipped method files and the step-methods table), L-SKILL beyond membership
(first skill ships with K1). L-KIND is active: its inputs are the shipped
policy tables (artifacts.toml carries the one story-kind table, walls.toml the
write grants).
"""

from __future__ import annotations

import importlib.util
import json
import re
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path

NEXT = Path(__file__).resolve().parents[2]
PLUGIN = NEXT / "plugins" / "vsdd-factory"
ROSTER = NEXT / "tests" / "lint" / "roster.toml"

sys.path.insert(0, str(PLUGIN / "py"))

_spec = importlib.util.spec_from_file_location("vsdd_build_lint", NEXT / "tools" / "build.py")
assert _spec is not None and _spec.loader is not None
build_mod = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(build_mod)

from vsdd.frontmatter import FrontmatterError, parse as _platform_parse  # noqa: E402

# The built-in tools of the platform. The MCP tools are not here: they are
# accepted only in the spelled forms below, from the servers the plugin bundles
# (.mcp.json) and the per-server tool lists of topics/tool-sets.md. Skill,
# AskUserQuestion, NotebookEdit and the task-list tools are deliberately
# absent, so listing one is rejected as an unknown tool.
BUILTIN_TOOLS = frozenset(
    [
        # READ, EDIT, SHELL, SPAWN (Agent), RESULT
        "Read", "Grep", "Glob", "Write", "Edit", "Bash", "Agent", "StructuredOutput",
        # WEB
        "WebSearch", "WebFetch",
    ]
)

# Per-server tool lists (topics/tool-sets.md). The servers are those the
# plugin's .mcp.json bundles, plus the project-side Storybook server, whose
# tools are accepted in the user-registration spelling only (step-5 plan D8).
MCP_SERVER_TOOLS: dict[str, frozenset[str]] = {
    "context7": frozenset({"resolve-library-id", "query-docs"}),
    "perplexity": frozenset({"perplexity_ask", "perplexity_search", "perplexity_research", "perplexity_reason"}),
    "tavily": frozenset({"tavily_search", "tavily_extract", "tavily_crawl", "tavily_map", "tavily_research"}),
    "playwright": frozenset(
        {
            "browser_navigate", "browser_navigate_back", "browser_snapshot", "browser_take_screenshot",
            "browser_click", "browser_type", "browser_fill_form", "browser_select_option",
            "browser_hover", "browser_press_key", "browser_wait_for", "browser_resize",
            "browser_console_messages", "browser_close",
        }
    ),
    "storybook": frozenset(
        {
            "list-all-documentation", "get-documentation", "get-documentation-for-story",
            "get-storybook-story-instructions", "preview-stories",
        }
    ),
}


def _bundled_servers() -> set[str]:
    """The server keys of the plugin's .mcp.json."""
    manifest = json.loads((PLUGIN / ".mcp.json").read_text())
    return set(manifest.get("mcpServers", {}))


MCP_BARE_NAMES = frozenset().union(*MCP_SERVER_TOOLS.values())

AGENT_SECTIONS = ("## Rules", "## Inputs", "## Method", "## Result")

IGNORED_FRONTMATTER_KEYS = ("permissionMode", "hooks", "mcpServers", "initialPrompt")

WORKFLOW_CALL_OPEN_RE = re.compile(r"(?<![\w.])workflow\s*\(")
# One complete quoted literal, closed before the argument separator: the only first argument
# whose workflow the lint can resolve to a roster edge (B6-R4). A concatenation, an
# interpolation or a bare name falls to the non-literal report instead.
WORKFLOW_LITERAL_ARG_RE = re.compile(r"\s*(['\"`])([A-Za-z0-9._:-]+)\1\s*[,)]")
DIRECT_AGENT_CALL_RE = re.compile(r"(?<![\w.])agent\s*\(")
# A tier setting written by hand: model or effort whose value holds a quoted literal anywhere
# before the value's next , or } — 'opus' outright and a choice between literals alike
# ({ model: big ? 'opus' : 'sonnet' }, C1-1). A data key whose value is carried rather than
# chosen ({ model: u.model }) is not a tier setting (B6-R7).
TIER_KEY_RE = re.compile(r"\b(model|effort)\s*:[^,}]*?(['\"`])")
TIER_TABLE_ASSIGN_RE = re.compile(r"\b(RUN\.tier|TIER|ROLE|OVERRIDE)\b[^=\n]*=(?!=)")
# The hand-written region may not reach the generated tables by other roads either: no
# subscripted write through RUN (RUN['tier'] = ..., which the assignment scan cannot see,
# C1-1) and no merge into it (Object.assign(RUN, ...)); and a tier: object key written by
# hand pins a tier wherever it sits, the two adoption forms carrying no object key (C1-1).
RUN_SUBSCRIPT_RE = re.compile(r"\bRUN\s*\[")
OBJECT_ASSIGN_RUN_RE = re.compile(r"\bObject\s*\.\s*assign\s*\(\s*RUN\b")
HANDWRITTEN_TIER_KEY_RE = re.compile(r"\btier\s*:")
# The one RUN.tier assignment a hand-written region may make: adopting the tier its caller or
# the survey passed — a dotted member chain on an input object (a.tier, s.config.tier_override),
# ending in `|| null` and naming no TIER, ROLE or OVERRIDE, so no quoted literal can fit
# either (B6-R7). RUN.tier = TIER.planner pins the whole run and stays rejected.
RUN_TIER_ADOPTION_RE = re.compile(r"RUN\.tier\s*=\s*([A-Za-z_$][\w$]*(?:\??\.[A-Za-z_$][\w$]*)+)\s*\|\|\s*null$")
TIER_NAME_RE = re.compile(r"\b(TIER|ROLE|OVERRIDE)\b")
REFERENCE_RE = re.compile(r"vsdd-factory:([A-Za-z0-9._-]+)")

# Keywords after which a / opens a regular expression rather than a division.
REGEX_KEYWORDS = frozenset(
    ["return", "typeof", "instanceof", "in", "of", "new", "delete", "void", "throw", "case", "do", "else", "yield", "await"]
)


def _regex_can_follow(out: list[str], index: int) -> bool:
    """True when a / at out[index] (the previous characters are in out) opens a regex literal:
    the last significant character decides, the way a JavaScript lexer splits division from a
    regular expression."""
    j = index - 1
    while j >= 0 and out[j] in " \t\r\n":
        j -= 1
    if j < 0:
        return True
    c = out[j]
    if c.isalnum() or c in "_$)]}\"'`" or c == ".":
        # After a word character check for the regex keywords (return /re/, await /re/).
        if c.isalnum() or c in "_$":
            k = j
            while k >= 0 and (out[k].isalnum() or out[k] in "_$"):
                k -= 1
            word = "".join(out[k + 1 : j + 1])
            if word in REGEX_KEYWORDS:
                return True
        return False
    return True


def _regex_end(js: str, start: int) -> int:
    """The index of the / that closes the regex literal opened at js[start], or -1: a regex
    never spans a line, and a / inside [...] does not close it."""
    i = start + 1
    in_class = False
    while i < len(js):
        c = js[i]
        if c == "\\":
            i += 2
            continue
        if c == "\n":
            return -1
        if c == "[":
            in_class = True
        elif c == "]":
            in_class = False
        elif c == "/" and not in_class:
            return i
        i += 1
    return -1


def _strip_comments_and_strings(js: str) -> str:
    """JavaScript with // and /* */ comments and the contents of ', " and ` strings blanked,
    so the agent and tier scans read the code only (B6-R7): a comment, a prompt string's
    'effort:' or a data key inside a string literal is not a tier setting. The body of a
    template literal is blanked, but a ${...} interpolation inside it is code: the scans see
    an agent( or a workflow( call, or a model: key, written there. A regular expression
    literal is opaque: its body is blanked, so a quote inside it (/'/g) reads as neither a
    string nor code. Characters keep their positions, so offsets in reports stay the file's."""
    out: list[str] = []
    i, n = 0, len(js)
    mode = None  # None, '//', '/*', the quote character of an open string, or a regex end
    interpolations: list[int] = []  # brace depth of the open interpolation of each open template
    while i < n:
        c = js[i]
        two = js[i : i + 2]
        if mode is None:
            if two == "//" or two == "/*":
                mode = two
                out.append("  ")
                i += 2
                continue
            if c in "'`\"":
                mode = c
                out.append(c)
                i += 1
                continue
            if c == "/" and _regex_can_follow(out, len(out)) and _regex_end(js, i) >= 0:
                end = _regex_end(js, i)
                out.append("/")
                out.append("".join("\n" if x == "\n" else " " for x in js[i + 1 : end]))
                out.append("/")
                i = end + 1
                while i < n and (js[i].isalpha() or js[i] in "_$"):
                    out.append(js[i])  # the flags (g, i, ...) belong to the literal
                    i += 1
                continue
            if c == "{":
                if interpolations:
                    interpolations[-1] += 1
                out.append(c)
                i += 1
                continue
            if c == "}":
                if interpolations and interpolations[-1] == 1:
                    interpolations.pop()
                    out.append("}")
                    mode = "`"
                    i += 1
                    continue
                if interpolations:
                    interpolations[-1] -= 1
                out.append(c)
                i += 1
                continue
            out.append(c)
            i += 1
            continue
        if mode == "//":
            out.append("\n" if c == "\n" else " ")
            if c == "\n":
                mode = None
            i += 1
            continue
        if mode == "/*":
            if two == "*/":
                out.append("  ")
                mode = None
                i += 2
                continue
            out.append("\n" if c == "\n" else " ")
            i += 1
            continue
        # inside a string or template literal (mode is the opening quote character)
        if c == "\\" and i + 1 < n:
            out.append("  ")  # a blanked escape pair keeps the length
            i += 2
            continue
        if mode == "`" and c == "$" and two == "${":
            interpolations.append(1)
            out.append("${")
            mode = None
            i += 2
            continue
        if mode in ("'", '"') and c == "\n":
            # An unescaped newline ends a ' or " string: JavaScript allows no string to span
            # lines, so a quote the stripper reaches through a misread / (a division read as a
            # regex, or the reverse) cannot swallow the rest of the file and hide a later call
            # from the scans (C1-1). A template literal still spans lines.
            out.append("\n")
            mode = None
            i += 1
            continue
        if c == mode:
            out.append(c)
            mode = None
            i += 1
            continue
        out.append("\n" if c == "\n" else " ")
        i += 1
    return "".join(out)


def load_roster(path: Path = ROSTER) -> dict:
    return tomllib.loads(path.read_text())


def _stamping_sources() -> dict:
    """The stamped texts, read once; the same fallback build.py uses."""
    keyrule = NEXT / "workflow-lib" / "finding-key.md"
    return {
        "severity": (NEXT / "workflow-lib" / "severity.md").read_text(),
        "keyrule": keyrule.read_text().rstrip("\n") if keyrule.is_file() else build_mod.FINDING_KEY_RULE,
    }


def _parse_frontmatter(text: str) -> tuple[dict, list[str]]:
    """The agent frontmatter, read by the shipped strict reader.

    The platform's reader (py/vsdd/frontmatter.py) keeps key case and refuses
    constructs outside the subset; a misread here would pass a file the
    platform loads differently (a wrong-case key silently loads the project's
    CLAUDE.md, a block scalar hides a long description).
    """
    try:
        return _platform_parse(text), []
    except FrontmatterError as exc:
        return {}, [f"frontmatter does not parse under the platform reader: {exc}"]


def _without_stamped_regions(text: str) -> str:
    out = text
    for opening, closing in (
        build_mod.SHARED_MARKERS,
        build_mod.SEVERITY_MARKERS,
        build_mod.KEYRULE_MARKERS,
    ):
        start = out.find(opening)
        end = out.find(closing)
        if start >= 0 and end > start:
            out = out[:start] + out[end + len(closing) :]
    return out


def _body_lines(text: str) -> int:
    """Lines of body: outside the frontmatter and outside the stamped regions."""
    lines = text.splitlines()
    if lines and lines[0].strip() == "---":
        for n, line in enumerate(lines[1:], 1):
            if line.strip() == "---":
                lines = lines[n + 1 :]
                break
    body = "\n".join(lines)
    body = _without_stamped_regions(body)
    return len("\n".join(body.split("\n")).rstrip("\n").split("\n"))


def _tool_problems(path: Path, name: str, tools: list[str]) -> list[str]:
    """One tools line against the built-in set and the bundled MCP servers."""
    problems: list[str] = []
    servers = _bundled_servers()
    for tool in tools:
        if tool in BUILTIN_TOOLS:
            continue
        if tool in MCP_BARE_NAMES:
            problems.append(
                f"{path.name}: tool {tool!r} is a bare MCP tool name, which the platform drops "
                "silently; list both spellings, mcp__plugin_vsdd-factory_<server>__<tool> and "
                "mcp__<server>__<tool> (topics/research-and-browser-tools.md)"
            )
            continue
        if tool.startswith("mcp__"):
            parts = tool.split("__")
            if len(parts) != 3 or not parts[2]:
                problems.append(f"{path.name}: tool {tool!r} is not a known tool (topics/tool-sets.md)")
                continue
            serverish, tool_name = parts[1], parts[2]
            plugin_form = serverish.startswith("plugin_vsdd-factory_")
            server = serverish[len("plugin_vsdd-factory_"):] if plugin_form else serverish
            if server not in MCP_SERVER_TOOLS or tool_name not in MCP_SERVER_TOOLS[server]:
                problems.append(f"{path.name}: tool {tool!r} is not a known tool (topics/tool-sets.md)")
                continue
            if plugin_form and server not in servers:
                problems.append(f"{path.name}: tool {tool!r} names a server the plugin does not bundle")
                continue
            if not plugin_form and server not in servers and server != "storybook":
                problems.append(f"{path.name}: tool {tool!r} names no bundled or project Storybook server")
                continue
            continue
        problems.append(f"{path.name}: tool {tool!r} is not a known tool (topics/tool-sets.md)")
    for server in sorted(servers & set(MCP_SERVER_TOOLS)):
        for tool_name in sorted(MCP_SERVER_TOOLS[server]):
            spellings = {
                f"mcp__plugin_vsdd-factory_{server}__{tool_name}",
                f"mcp__{server}__{tool_name}",
            }
            present = spellings & set(tools)
            if len(present) == 1:
                problems.append(
                    f"{path.name}: {present.pop()!r} needs the other spelling too, "
                    f"mcp__plugin_vsdd-factory_{server}__{tool_name} and mcp__{server}__{tool_name} "
                    "(topics/research-and-browser-tools.md: list both)"
                )
    return problems


def agent_problems(path: Path, roster: dict, lib: dict) -> list[str]:
    """L-AGENT (table:lint-checks) for one agent file; roster is the loaded roster.toml."""
    problems: list[str] = []
    name = path.stem
    text = path.read_text()
    fm, fm_problems = _parse_frontmatter(text)
    problems.extend(fm_problems)
    if fm_problems:
        return problems

    agents = roster.get("agents", {})
    tiers = roster.get("tiers", {})
    budgets = roster.get("budgets", {})
    tier_name = agents.get(name)

    if fm.get("name") != name:
        problems.append(f"{path.name}: frontmatter name {fm.get('name')!r} must equal the file name {name!r}")
    description = fm.get("description") or ""
    if not description:
        problems.append(f"{path.name}: description is missing")
    elif len(description) > 200:
        problems.append(f"{path.name}: description is {len(description)} characters, at most 200")

    if tier_name is None:
        problems.append(f"{path.name}: agent is not in the roster's [agents]")
        tier = {}
    else:
        tier = tiers.get(tier_name, {})
    if tier:
        if fm.get("model") != tier.get("model") or fm.get("effort") != tier.get("effort"):
            problems.append(
                f"{path.name}: model/effort {fm.get('model')}/{fm.get('effort')} must be the "
                f"{tier_name} tier's {tier.get('model')}/{tier.get('effort')} (roster [tiers])"
            )
        color = tier.get("color")
        if color and fm.get("color") != color:
            problems.append(f"{path.name}: color must be {color} for the {tier_name} tier, got {fm.get('color')!r}")
    if fm.get("effort") == "low" and tier_name != "clerk":
        problems.append(f"{path.name}: effort low is accepted for the clerk tier only")

    if fm.get("omitClaudeMd") is not True:
        problems.append(f"{path.name}: omitClaudeMd: true must be set")
    for ignored in IGNORED_FRONTMATTER_KEYS:
        if ignored in fm:
            problems.append(f"{path.name}: frontmatter key {ignored} is silently ignored by the platform; remove it")
    allowed_keys = {"name", "description", "model", "effort", "tools", "color", "omitClaudeMd"}
    if name == "spike-runner":
        allowed_keys |= {"isolation"}
    for key in fm:
        if key not in allowed_keys:
            problems.append(f"{path.name}: frontmatter key {key} is not in the standard agent template")

    tools = [t.strip() for t in str(fm.get("tools") or "").split(",") if t.strip()]
    if not tools:
        problems.append(f"{path.name}: tools is missing; it is never omitted")
    problems.extend(_tool_problems(path, name, tools))
    if name not in ("spike-runner", "clerk") and "Agent" not in tools:
        problems.append(f"{path.name}: every agent except spike-runner and clerk lists the Agent tool")

    for section in AGENT_SECTIONS:
        if section not in text:
            problems.append(f"{path.name}: missing the section '{section}' the standard agent template requires")

    # The whole-file budget counts the file with its stamped regions removed:
    # frontmatter plus a body of at most agent_body_lines (agents-skills.md 2.7).
    whole = budgets.get("agent_file_lines")
    unstamped = _without_stamped_regions(text).rstrip("\n")
    if whole is not None and len(unstamped.splitlines()) > whole:
        problems.append(
            f"{path.name}: {len(unstamped.splitlines())} lines without the stamped regions is over "
            f"the roster budget of {whole}"
        )
    body_cap = budgets.get("agent_body_lines")
    if body_cap is not None:
        body = _body_lines(text)
        if body > body_cap:
            problems.append(f"{path.name}: body is {body} lines, over the budget of {body_cap} (stamped regions excluded)")

    if tier_name is not None:
        wanted = build_mod._stamp_shared(text, build_mod._reviewer_variant(name))
        if build_mod.is_reviewer(name, roster, text):
            wanted = build_mod._stamp_region(wanted, build_mod.SEVERITY_MARKERS, lib["severity"])
            wanted = build_mod._stamp_region(wanted, build_mod.KEYRULE_MARKERS, lib["keyrule"])
        if wanted != text:
            problems.append(f"{path.name}: stamped block drifted; run tools/build.py (the shared block is compared byte for byte)")
    return problems


def _hand_written_region(text: str) -> str:
    """The workflow file with its stamped prelude removed."""
    stamped_at = text.find(build_mod.PRELUDE_MARKERS[0])
    stamped_end = text.find(build_mod.PRELUDE_MARKERS[1])
    if 0 <= stamped_at < stamped_end:
        return text[:stamped_at] + text[stamped_end + len(build_mod.PRELUDE_MARKERS[1]) :]
    return text


def workflow_problems(path: Path, roster: dict) -> list[str]:
    """L-ROSTER membership, the call edges of L-XREF and the tier-override rule."""
    problems: list[str] = []
    text = path.read_text()
    name = path.stem
    workflows = roster.get("workflows", {})
    if name not in workflows:
        problems.append(f"{path.name}: workflow is not in the roster's [workflows]")
    meta = text.split("}", 1)[0] if text.startswith("export const meta") else ""
    meta_name = re.search(r"name:\s*['\"]?([A-Za-z0-9._-]+)['\"]?", meta)
    if meta_name and meta_name.group(1) != name:
        problems.append(f"{path.name}: meta name {meta_name.group(1)!r} must equal the file name {name!r}")

    # The stamped prelude carries runStories(), whose workflow('vsdd-factory:story') call belongs
    # to the caller, not to the file: only the hand-written region may hold call edges. In the
    # hand-written region runStories(...) is itself a call of story: it is how workflows call it.
    # The scans run over the body with comments and string literals blanked (B6-R7): a prompt
    # string's 'effort:' or a comment naming the agent is not a tier setting of the workflow.
    body = _hand_written_region(text)
    js = _strip_comments_and_strings(body)
    calls = set()
    # The open must be code (it matches in js, where strings and comments are blanked), but its
    # argument is the string literal itself, so the literal is matched in the original body at
    # the same offset: the stripper preserves every character's position.
    for match in WORKFLOW_CALL_OPEN_RE.finditer(js):
        literal = WORKFLOW_LITERAL_ARG_RE.match(body, match.end())
        if literal is None:
            problems.append(
                f"{path.name}: workflow()'s first argument is not a literal workflow name "
                f"(expression at offset {match.start()}), so the lint cannot check the edge "
                "against the roster"
            )
            continue
        calls.add(literal.group(2).removeprefix("vsdd-factory:"))
    if re.search(r"(?<![\w.])runStories\s*\(", js):
        calls.add("story")

    called_by_others = {called for entry in workflows.values() for called in entry.get("calls", [])}
    for target in sorted(calls):
        if target not in workflows:
            problems.append(f"{path.name}: calls workflow {target!r}, which is not in the roster's [workflows]")
        elif target not in workflows.get(name, {}).get("calls", []):
            problems.append(
                f"{path.name}: calls workflow {target!r}, which the roster does not list for {name!r} "
                "(L-XREF: the roster's [workflows] is the one call table)"
            )
    if name in called_by_others and calls:
        problems.append(
            f"{path.name}: a workflow another workflow calls contains no workflow() call of its own "
            f"(found {sorted(calls)})"
        )

    # The tiers are the roster's (decision D5): the hand-written region may not
    # set a model or effort itself, call an agent directly, assign the tier,
    # role or override tables the prelude generates, name them at all (an alias
    # would rename them past the assignment scan, C1-1), reach RUN other than
    # through its fields (a subscript or a merge would carry a tier unseen),
    # or write a tier: key. The one exception is the RUN.tier adoption the
    # skeletons use, which takes the tier from the caller or the survey
    # (RUN_TIER_ADOPTION_RE).
    for match in DIRECT_AGENT_CALL_RE.finditer(js):
        problems.append(f"{path.name}: a direct agent( call at offset {match.start()}); steps go through step(), relay() or gated()")
    for match in TIER_KEY_RE.finditer(js):
        problems.append(f"{path.name}: the tier settings are the roster's (offset {match.start()}: {match.group(0)!r}); a workflow step takes model and effort from the generated table")
    for match in TIER_TABLE_ASSIGN_RE.finditer(js):
        line_end = js.find("\n", match.start())
        line = js[match.start() : line_end if line_end >= 0 else len(js)].strip()
        adopted = RUN_TIER_ADOPTION_RE.match(line)
        if match.group(1) == "RUN.tier" and adopted and not TIER_NAME_RE.search(adopted.group(1)):
            continue
        problems.append(f"{path.name}: the tier settings are the roster's (offset {match.start()}: {line!r}); a workflow step takes model and effort from the generated table")
    for match in TIER_NAME_RE.finditer(js):
        problems.append(
            f"{path.name}: the tier settings are the roster's (offset {match.start()}: {match.group(1)}); "
            "the prelude alone reads TIER, ROLE or OVERRIDE"
        )
    for regex, detail in (
        (RUN_SUBSCRIPT_RE, "write RUN.tier only as the adoption line"),
        (OBJECT_ASSIGN_RUN_RE, "write RUN.tier only as the adoption line"),
        (HANDWRITTEN_TIER_KEY_RE, "a step takes its tier from the generated table"),
    ):
        for match in regex.finditer(js):
            problems.append(
                f"{path.name}: the tier settings are the roster's (offset {match.start()}: {match.group(0)!r}); {detail}"
            )
    return problems


def reference_problems(paths: list[Path], roster: dict) -> list[str]:
    """The reference half of L-XREF, over the files this lint reads: every
    vsdd-factory:<name> must resolve to a roster agent, workflow or skill."""
    known = (
        set(roster.get("agents", {}))
        | set(roster.get("workflows", {}))
        | set(roster.get("skills", {}))
    )
    problems: list[str] = []
    for path in paths:
        for name in REFERENCE_RE.findall(path.read_text()):
            if name not in known:
                problems.append(f"{path.name}: references vsdd-factory:{name}, which resolves to nothing in the roster")
    return problems


def manifest_problems(plugin: Path, roster: dict) -> list[str]:
    """The roster-linked and existence parts of L-MAN; the field literals live in tools/check.py."""
    problems: list[str] = []
    path = plugin / ".claude-plugin" / "plugin.json"
    try:
        manifest = json.loads(path.read_text())
    except (OSError, ValueError) as exc:
        return [f"{path}: unreadable manifest ({exc})"]
    floor = roster.get("platform", {}).get("min_claude_code")
    engines = manifest.get("engines", {}).get("claude-code")
    if floor is not None and engines != f">={floor}":
        problems.append(f"plugin.json engines.claude-code {engines!r} must be '>={floor}', the roster's [platform] floor")
    icon = manifest.get("icon")
    if icon:
        if not (plugin / icon.removeprefix("./")).is_file():
            problems.append(f"plugin.json icon {icon!r} does not exist")
    elif "icon" in manifest:
        problems.append("plugin.json icon is empty")
    else:
        problems.append("plugin.json has no icon")
    if not (plugin / "README.md").is_file():
        problems.append("missing plugins/vsdd-factory/README.md")
    return problems


def membership_problems(plugin: Path, roster: dict) -> list[str]:
    """L-ROSTER, disk side: nothing on disk is absent from the roster."""
    problems: list[str] = []
    agents_dir, skills_dir, workflows_dir = plugin / "agents", plugin / "skills", plugin / "workflows"
    for path in sorted(agents_dir.glob("*.md")) if agents_dir.is_dir() else []:
        if path.stem not in roster.get("agents", {}):
            problems.append(f"{path}: agent is not in the roster's [agents]")
    if skills_dir.is_dir():
        for path in sorted(skills_dir.iterdir()):
            if path.is_dir() and not path.name.startswith(".") and path.name not in roster.get("skills", {}):
                problems.append(f"{path}: skill is not in the roster's [skills]")
    for path in sorted(workflows_dir.glob("*.js")) if workflows_dir.is_dir() else []:
        if path.stem not in roster.get("workflows", {}):
            problems.append(f"{path}: workflow is not in the roster's [workflows]")
    return problems


def absence_problems(plugin: Path, roster: dict) -> list[str]:
    """L-ROSTER, roster side: nothing in the roster is absent from disk (B6-R12), the mirror of
    membership_problems. Active against the real plugin only once every roster file exists;
    the fixture tests below hold it meanwhile."""
    problems: list[str] = []
    agents_dir, skills_dir, workflows_dir = plugin / "agents", plugin / "skills", plugin / "workflows"
    for name in roster.get("agents", {}):
        if not (agents_dir / f"{name}.md").is_file():
            problems.append(f"{agents_dir / name}.md: the roster's agent {name!r} has no agent file")
    for name in roster.get("skills", {}):
        if not (skills_dir / name).is_dir():
            problems.append(f"{skills_dir / name}: the roster's skill {name!r} has no skill directory")
    for name in roster.get("workflows", {}):
        if not (workflows_dir / f"{name}.js").is_file():
            problems.append(f"{workflows_dir / name}.js: the roster's workflow {name!r} has no workflow file")
    return problems


def roster_problems(roster: dict) -> list[str]:
    """The roster's own integrity: the budgets it holds the files to are coherent,
    and every name it carries points at another part of itself."""
    problems: list[str] = []
    budgets = roster.get("budgets", {})
    for key in ("model_visible_skills", "listed_description_chars", "agent_file_lines", "agent_body_lines", "skill_file_lines", "method_file_lines"):
        if key not in budgets:
            problems.append(f"roster [budgets] is missing {key}")
    tiers = roster.get("tiers", {})
    agents = roster.get("agents", {})
    workflows = roster.get("workflows", {})
    skills = roster.get("skills", {})
    overrides = roster.get("overrides", {})
    for agent, tier in agents.items():
        if tier not in tiers:
            problems.append(f"roster [agents] {agent!r} names tier {tier!r}, which [tiers] does not define")
    for workflow, entry in workflows.items():
        for called in entry.get("calls", []):
            if called not in workflows:
                problems.append(f"roster [workflows] {workflow!r} calls {called!r}, which is not a roster workflow")
    called_by_others = {called for entry in workflows.values() for called in entry.get("calls", [])}
    for called in sorted(called_by_others):
        if workflows.get(called, {}).get("calls"):
            problems.append(f"roster [workflows]: {called!r} is called by another workflow, so its own calls must be []")
    for workflow, tier in overrides.items():
        if tier not in tiers:
            problems.append(f"roster [overrides] {workflow!r} names tier {tier!r}, which [tiers] does not define")
        target = workflow.split("/", 1)[0]
        if target not in workflows:
            problems.append(f"roster [overrides] {workflow!r} names no roster workflow")
    for skill, mode in skills.items():
        if mode not in ("person", "model", "both"):
            problems.append(f"roster [skills] {skill!r} is {mode!r}; a skill is person-, model-visible or both")
    visible = [name for name, mode in skills.items() if mode != "person"]
    cap = budgets.get("model_visible_skills")
    if cap is not None and len(visible) > cap:
        problems.append(f"roster [skills] lists {len(visible)} model-visible skills, over the budget of {cap}")
    return problems


# L-KIND (table:lint-checks): for every story kind in artifacts.toml, the agent of its
# implement stage (and of its tests stage, where it has one) holds, in walls.toml, a write
# grant for every class the method's output needs: the implementer source and manifests; the
# test-writer tests; devops ci, repo-config, manifests, and for repo-setup new files in source
# and tests; the verifier harness and tests; the technical-writer docs. Each need is a
# (path class, method) pair: the method is empty for an unconditional need, and names the
# method when the class is needed only for stories whose resolved stage method is that one
# (repo-setup). A need is met by a write entry of the same class whose own @method qualifier,
# if any, matches. A conditional qualifier (:create new files only, :before-red before the red
# gate) does not meet an unconditional need; :frozen does, because freezing restricts which
# tests file, not whether the agent may write the class.
METHOD_WRITE_NEEDS: dict[str, set[tuple[str, str]]] = {
    "implementer": {("source", ""), ("manifests", "")},
    "technical-writer": {("docs", "")},
    "devops": {("ci", ""), ("repo-config", ""), ("manifests", ""), ("source", "repo-setup"), ("tests", "repo-setup")},
    "verifier": {("harness", ""), ("tests", "")},
}
TESTS_WRITE_NEEDS = {"test-writer": {("tests", "")}}

CONDITIONAL_QUALIFIERS = {"create", "before-red"}


def _grant_parts(entry: str) -> tuple[str, str, str]:
    """('source', 'create', 'repo-setup') for the walls.toml write entry 'source:create@repo-setup'."""
    head, _, method = entry.partition("@")
    cls, _, qualifier = head.partition(":")
    return cls, qualifier, method


def kind_grant_problems(policy: Path) -> list[str]:
    """L-KIND over the shipped policy tables: policy is the plugin's py/vsdd/policy directory."""
    problems: list[str] = []
    kinds = tomllib.loads((policy / "artifacts.toml").read_text()).get("story-kind", [])
    if not kinds:
        return [f"{policy}/artifacts.toml: carries no [[story-kind]] rows, so L-KIND checks nothing"]
    walls = tomllib.loads((policy / "walls.toml").read_text())
    grants_of = {
        agent[len("vsdd-factory:") :]: {_grant_parts(entry) for entry in row.get("write", [])}
        for agent, row in walls.items()
        if agent.startswith("vsdd-factory:")
    }
    for kind in kinds:
        kid = kind.get("id", "?")
        stages = (
            ("implement", kind.get("implement_agent", ""), kind.get("implement_method", ""), METHOD_WRITE_NEEDS),
            ("tests", kind.get("tests_agent", ""), kind.get("tests_method", ""), TESTS_WRITE_NEEDS),
        )
        for stage, agent, method, needs_table in stages:
            if not agent:
                if stage == "implement":
                    problems.append(f"story kind {kid!r}: names no implement agent")
                continue
            if agent not in needs_table:
                problems.append(
                    f"story kind {kid!r}: the L-KIND table has no write needs for the {stage} agent "
                    f"{agent!r}; extend tests/lint/test_roster_lint.py"
                )
                continue
            grants = grants_of.get(agent, set())
            missing = [
                f"{cls!r}" + (f" in a {want_method} story" if want_method else "")
                for cls, want_method in sorted(needs_table[agent])
                if not any(
                    g_cls == cls
                    and g_method in ("", want_method)
                    and (want_method or qualifier not in CONDITIONAL_QUALIFIERS)
                    for g_cls, qualifier, g_method in grants
                )
            ]
            if missing:
                problems.append(
                    f"story kind {kid!r}: the {stage} agent {agent!r} holds no walls.toml write grant "
                    f"for {', '.join(missing)}"
                )
    return problems


def walls_row_problems(policy: Path, roster: dict) -> list[str]:
    """L-AGENT's closing clause: every agent has a row in walls.toml and every row has an agent."""
    problems: list[str] = []
    walls = tomllib.loads((policy / "walls.toml").read_text())
    rows = {name[len("vsdd-factory:"):] for name in walls if name.startswith("vsdd-factory:")}
    agents = set(roster.get("agents", {}))
    missing = sorted(agents - rows)
    extra = sorted(rows - agents)
    if missing:
        problems.append(f"walls.toml has no row for the roster agent(s): {', '.join(missing)}")
    if extra:
        problems.append(f"walls.toml has rows no roster agent matches: {', '.join(extra)}")
    return problems


def check_all(plugin: Path, roster: dict, lib: dict) -> list[str]:
    problems: list[str] = []
    problems.extend(manifest_problems(plugin, roster))
    problems.extend(membership_problems(plugin, roster))
    problems.extend(roster_problems(roster))
    policy = plugin / "py" / "vsdd" / "policy"
    if policy.is_dir():
        problems.extend(kind_grant_problems(policy))
        problems.extend(walls_row_problems(policy, roster))
    else:
        problems.append(f"{policy}: the shipped policy tables are missing, so L-KIND and the walls-row check check nothing")
    agent_paths = sorted((plugin / "agents").glob("*.md")) if (plugin / "agents").is_dir() else []
    workflow_paths = sorted((plugin / "workflows").glob("*.js")) if (plugin / "workflows").is_dir() else []
    problems.extend(reference_problems(agent_paths + workflow_paths, roster))
    for path in agent_paths:
        problems.extend(agent_problems(path, roster, lib))
    for path in workflow_paths:
        problems.extend(workflow_problems(path, roster))
    return problems


class TestAgentLint(unittest.TestCase):
    """L-AGENT against fixtures; the stamping runs through tools/build.py itself."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="vsdd-rosterlint-")
        self.addCleanup(self._tmp.cleanup)
        self.plugin = Path(self._tmp.name) / "vsdd-factory"
        (self.plugin / "agents").mkdir(parents=True)
        self.lib = _stamping_sources()
        self.roster = load_roster()

    def write_agent(self, text: str, stem: str | None = None, stamp: bool = True) -> Path:
        name = stem or re.search(r"^name:\s*(\S+)", text, re.M).group(1)
        if stamp:
            text = build_mod._stamp_shared(text, build_mod._reviewer_variant(name))
        path = self.plugin / "agents" / f"{name}.md"
        path.write_text(text, encoding="utf-8")
        return path

    @staticmethod
    def agent_text(**overrides: str) -> str:
        parts = {
            "name": "test-writer",
            "description": "Writes the failing tests the story needs. Use for a story's red stage.",
            "model": "sonnet",
            "effort": "xhigh",
            "tools": "Read, Grep, Glob, Write, Edit, Bash, Agent, StructuredOutput",
            "color": "green",
            "omitClaudeMd": "true",
        }
        parts.update(overrides)
        frontmatter = "\n".join(f"{k}: {v}" for k, v in parts.items())
        return (
            f"---\n{frontmatter}\n---\n\n"
            "You are the test-writer of a vsdd-factory project. You write the failing tests first.\n\n"
            "## Rules\n- Every criterion gets a test.\n\n"
            "## Inputs\n- The story file and its contracts.\n\n"
            "## Method\nYour task names a method. Read `${CLAUDE_PLUGIN_ROOT}/methods/<name>.md` before you start.\n\n"
            "## Result\nFill in the result schema you were given.\n"
        )

    def stamp_reviewer(self, path: Path) -> None:
        text = path.read_text()
        text = build_mod._stamp_region(text, build_mod.SEVERITY_MARKERS, self.lib["severity"])
        text = build_mod._stamp_region(text, build_mod.KEYRULE_MARKERS, self.lib["keyrule"])
        path.write_text(text)

    def test_correct_builder_agent_passes(self) -> None:
        path = self.write_agent(self.agent_text())
        self.assertEqual(agent_problems(path, self.roster, self.lib), [])

    def test_correct_reviewer_agent_passes(self) -> None:
        path = self.write_agent(
            self.agent_text(name="adversary", model="opus", color="red", tools="Read, Write, Agent, StructuredOutput")
        )
        self.stamp_reviewer(path)
        self.assertEqual(agent_problems(path, self.roster, self.lib), [])

    def test_correct_clerk_passes_with_low_effort_and_no_agent_tool(self) -> None:
        path = self.write_agent(
            self.agent_text(name="clerk", model="sonnet", effort="low", tools="Bash, StructuredOutput", color="yellow")
        )
        self.assertEqual(agent_problems(path, self.roster, self.lib), [])

    def test_low_effort_outside_the_clerk_tier_is_rejected(self) -> None:
        path = self.write_agent(self.agent_text(effort="low"))
        self.assertTrue(any("clerk tier only" in p for p in agent_problems(path, self.roster, self.lib)))

    def test_wrong_model_for_the_tier_is_rejected(self) -> None:
        path = self.write_agent(self.agent_text(model="opus"))
        self.assertTrue(any("must be the builder tier" in p for p in agent_problems(path, self.roster, self.lib)))

    def test_wrong_colour_for_the_tier_is_rejected(self) -> None:
        path = self.write_agent(self.agent_text(color="blue"))
        self.assertTrue(any("color must be green" in p for p in agent_problems(path, self.roster, self.lib)))

    def test_the_clerk_colour_comes_from_the_roster(self) -> None:
        # The clerk's colour rides in its [tiers] row, so the lint checks it like any other tier's.
        path = self.write_agent(
            self.agent_text(name="clerk", model="sonnet", effort="low", tools="Bash, StructuredOutput", color="green")
        )
        self.assertTrue(any("color must be yellow" in p for p in agent_problems(path, self.roster, self.lib)))

    def test_missing_omitclaudemd_is_rejected(self) -> None:
        path = self.write_agent(self.agent_text(omitClaudeMd="false"))
        self.assertTrue(any("omitClaudeMd: true" in p for p in agent_problems(path, self.roster, self.lib)))

    def test_a_lowercase_omitclaudemd_key_is_rejected(self) -> None:
        # The platform reads the exact key; 'omitclaudemd' silently loads the
        # project's CLAUDE.md into the agent.
        text = self.agent_text().replace("omitClaudeMd: true", "omitclaudemd: true")
        path = self.write_agent(text, stamp=True)
        problems = agent_problems(path, self.roster, self.lib)
        self.assertTrue(any("omitClaudeMd: true must be set" in p for p in problems), problems)
        self.assertTrue(any("omitclaudemd" in p and "standard agent template" in p for p in problems), problems)

    def test_a_block_scalar_description_is_rejected(self) -> None:
        text = self.agent_text().replace(
            "description: Writes the failing tests",
            "description: >\n  Writes the failing tests",
        )
        path = self.write_agent(text, stamp=True)
        problems = agent_problems(path, self.roster, self.lib)
        self.assertTrue(any("does not parse" in p for p in problems), problems)

    def test_a_quoted_model_value_is_parsed_as_the_platform_reads_it(self) -> None:
        text = self.agent_text().replace("model: sonnet", 'model: "sonnet"')
        path = self.write_agent(text)
        self.assertEqual(agent_problems(path, self.roster, self.lib), [])

    def test_platform_ignored_keys_are_rejected(self) -> None:
        path = self.write_agent(self.agent_text())
        self.assertEqual(agent_problems(path, self.roster, self.lib), [])  # control: the file is clean
        path.write_text(path.read_text().replace("omitClaudeMd: true", "omitClaudeMd: true\npermissionMode: default", 1))
        problems = agent_problems(path, self.roster, self.lib)
        self.assertTrue(any("permissionmode" in p.lower() for p in problems), problems)

    def test_unknown_tool_is_rejected(self) -> None:
        path = self.write_agent(self.agent_text(tools="Read, NotebookEdit"))
        self.assertTrue(any("NotebookEdit" in p and "not a known tool" in p for p in agent_problems(path, self.roster, self.lib)))

    def test_a_bare_mcp_tool_name_is_rejected(self) -> None:
        path = self.write_agent(self.agent_text(tools='"Read, browser_navigate"'))
        problems = agent_problems(path, self.roster, self.lib)
        self.assertTrue(any("bare MCP tool name" in p for p in problems), problems)

    def test_both_spellings_of_a_bundled_server_tool_are_accepted(self) -> None:
        both = (
            "mcp__plugin_vsdd-factory_context7__query-docs, mcp__context7__query-docs, "
            "mcp__plugin_vsdd-factory_playwright__browser_navigate, mcp__playwright__browser_navigate"
        )
        path = self.write_agent(self.agent_text(tools=f'"Read, Write, Agent, {both}"'))
        self.assertEqual(agent_problems(path, self.roster, self.lib), [])

    def test_one_spelling_without_the_other_is_rejected(self) -> None:
        path = self.write_agent(self.agent_text(tools='"Read, mcp__context7__query-docs"'))
        problems = agent_problems(path, self.roster, self.lib)
        self.assertTrue(any("needs the other spelling too" in p for p in problems), problems)

    def test_the_second_spelling_alone_is_rejected(self) -> None:
        path = self.write_agent(self.agent_text(tools='"Read, mcp__plugin_vsdd-factory_context7__query-docs"'))
        problems = agent_problems(path, self.roster, self.lib)
        self.assertTrue(any("needs the other spelling too" in p for p in problems), problems)

    def test_the_storybook_spelling_alone_is_accepted(self) -> None:
        # Storybook is a project-side server (step-5 plan D8): only the user
        # registration spelling exists, and it needs no plugin-form twin.
        path = self.write_agent(self.agent_text(tools='"Read, Agent, mcp__storybook__preview-stories"'))
        self.assertEqual(agent_problems(path, self.roster, self.lib), [])

    def test_implementer_without_the_agent_tool_is_rejected(self) -> None:
        path = self.write_agent(self.agent_text(name="implementer", tools="Read, Write, Bash, StructuredOutput"))
        self.assertTrue(any("Agent tool" in p for p in agent_problems(path, self.roster, self.lib)))

    def test_filename_must_equal_the_name_field(self) -> None:
        path = self.write_agent(self.agent_text(), stem="verifier", stamp=False)
        path = path.with_name("wrong-name.md")
        path.write_text(self.agent_text())
        self.assertTrue(any("must equal the file name" in p for p in agent_problems(path, self.roster, self.lib)))

    def test_overlong_description_is_rejected(self) -> None:
        path = self.write_agent(self.agent_text(description="x" * 201))
        self.assertTrue(any("at most 200" in p for p in agent_problems(path, self.roster, self.lib)))

    def test_body_over_sixty_lines_is_rejected(self) -> None:
        filler = "\n".join(f"- rule {n}" for n in range(70))
        path = self.write_agent(self.agent_text() + "## Extra body\n" + filler + "\n")
        self.assertTrue(any("body is" in p and "over the budget of 60" in p for p in agent_problems(path, self.roster, self.lib)))

    def test_a_body_at_the_budget_passes_and_one_line_over_fails(self) -> None:
        base = self.agent_text()
        room = 60 - _body_lines(base)
        filler = "\n".join(f"- rule {n}" for n in range(room))
        at_limit = self.write_agent(base + filler + "\n")
        self.assertEqual(_body_lines(at_limit.read_text()), 60)
        self.assertEqual(agent_problems(at_limit, self.roster, self.lib), [])
        one_over = self.write_agent(base + filler + "\n- one more\n")
        self.assertTrue(any("body is 61 lines" in p for p in agent_problems(one_over, self.roster, self.lib)))

    def test_the_whole_file_budget_counts_the_file_without_its_stamped_regions(self) -> None:
        # A reviewer's severity table and key rule ride on top of the same
        # budget; the stamped regions are not counted (agents-skills.md 2.7).
        base = self.agent_text(name="adversary", model="opus", color="red", tools="Read, Write, Agent, StructuredOutput")
        unstamped_lines = len(_without_stamped_regions(base).splitlines())
        room = 70 - unstamped_lines - 1
        filler = "\n".join(f"- rule {n}" for n in range(max(room, 0)))
        path = self.write_agent(base + filler + "\n")
        self.assertLessEqual(len(_without_stamped_regions(path.read_text()).rstrip("\n").splitlines()), 70)
        self.stamp_reviewer(path)
        self.assertEqual(agent_problems(path, self.roster, self.lib), [])

    def test_the_agent_file_topic_example_passes_stamped(self) -> None:
        # The design's complete example (topics/agent-file.md), stamped, must
        # pass the lint as it stands.
        example_path = NEXT.parent / "rewrite" / "spec" / "topics" / "agent-file.md"
        example = example_path.read_text().split("## The complete example", 1)[1]
        text = example.split("```markdown", 1)[1].rsplit("```", 1)[0].strip("\n") + "\n"
        path = self.write_agent(text)
        self.stamp_reviewer(path)
        self.assertEqual(agent_problems(path, self.roster, self.lib), [])

    def test_shared_block_drift_is_rejected(self) -> None:
        path = self.write_agent(self.agent_text())
        path.write_text(path.read_text().replace("Ground rules", "Ground ruleses"))
        self.assertTrue(any("stamped block drifted" in p for p in agent_problems(path, self.roster, self.lib)))

    def test_reviewer_without_the_severity_stamp_is_rejected(self) -> None:
        path = self.write_agent(
            self.agent_text(name="adversary", model="opus", color="red", tools="Read, Write, Agent, StructuredOutput"),
            stamp=True,
        )
        self.assertTrue(any("stamped block drifted" in p for p in agent_problems(path, self.roster, self.lib)))
        self.stamp_reviewer(path)
        self.assertEqual(agent_problems(path, self.roster, self.lib), [])

    def test_missing_template_section_is_rejected(self) -> None:
        text = self.agent_text()
        path = self.write_agent(text.split("## Result")[0], stamp=True)
        self.assertTrue(any("## Result" in p for p in agent_problems(path, self.roster, self.lib)))


class TestBuildStampingSeam(unittest.TestCase):
    """The build's stamping and the lint make one reviewer decision (B6 seam):
    a reviewer agent the build has stamped satisfies the lint with no problems."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="vsdd-seam-")
        self.addCleanup(self._tmp.cleanup)
        self.plugin = Path(self._tmp.name) / "vsdd-factory"
        self.agents_dir = self.plugin / "agents"
        self.agents_dir.mkdir(parents=True)

    def test_a_reviewer_stamped_by_the_build_passes_the_lint(self) -> None:
        original = build_mod.AGENTS_DIR
        build_mod.AGENTS_DIR = self.agents_dir
        try:
            text = TestAgentLint.agent_text(
                name="adversary", model="opus", color="red", tools="Read, Write, Agent, StructuredOutput"
            )
            path = self.agents_dir / "adversary.md"
            path.write_text(text, encoding="utf-8")
            problems: list[str] = []
            drifted: list[str] = []
            build_mod._stamp_agents(problems, drifted, check_only=False)
            self.assertEqual(problems, [])
            self.assertIn(build_mod.SEVERITY_MARKERS[0], path.read_text())
            roster = load_roster()
            self.assertEqual(agent_problems(path, roster, _stamping_sources()), [])
        finally:
            build_mod.AGENTS_DIR = original


class TestWorkflowLint(unittest.TestCase):
    """The call edges of L-XREF and the tier-override rule, with the stamped
    prelude excluded from the scan."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="vsdd-rosterlint-")
        self.addCleanup(self._tmp.cleanup)
        self.plugin = Path(self._tmp.name) / "vsdd-factory"
        (self.plugin / "workflows").mkdir(parents=True)
        self.roster = load_roster()

    def write_workflow(self, stem: str, body: str) -> Path:
        path = self.plugin / "workflows" / f"{stem}.js"
        path.write_text(
            f"export const meta = {{\n  name: '{stem}',\n  phases: [],\n}}\n"
            f"{build_mod.PRELUDE_MARKERS[0]}\n{build_mod.PRELUDE_MARKERS[1]}\n" + body,
            encoding="utf-8",
        )
        return path

    def test_story_that_calls_nothing_passes(self) -> None:
        path = self.write_workflow("story", "RUN.wf = 'story'\nreturn {}\n")
        self.assertEqual(workflow_problems(path, self.roster), [])

    def test_wave_may_call_story(self) -> None:
        path = self.write_workflow("wave", "await workflow('vsdd-factory:story', {})\nreturn {}\n")
        self.assertEqual(workflow_problems(path, self.roster), [])

    def test_a_backtick_call_of_a_forbidden_edge_is_rejected(self) -> None:
        # B6-R4: backticks are quoted literals, and the edge rule reads them like any other
        # quoting; wave's roster row lists story alone, so a backticked release call is rejected.
        path = self.write_workflow("wave", "await workflow(`vsdd-factory:release`, {})\nreturn {}\n")
        self.assertTrue(any("does not list for 'wave'" in p for p in workflow_problems(path, self.roster)))

    def test_a_space_before_the_parenthesis_is_still_an_edge(self) -> None:
        # B6-R4: whitespace between the callee and its parenthesis hid the edge from the
        # scan, so a forbidden call passed silently; the space form is read like any other.
        path = self.write_workflow("wave", "await workflow ('vsdd-factory:release', {})\nreturn {}\n")
        self.assertTrue(any("does not list for 'wave'" in p for p in workflow_problems(path, self.roster)))
        path = self.write_workflow("wave", "await workflow ('vsdd-factory:story', {})\nreturn {}\n")
        self.assertEqual(workflow_problems(path, self.roster), [])

    def test_a_non_literal_quoted_first_argument_is_reported(self) -> None:
        # B6-R4: 'vsdd-factory:' + name and `vsdd-factory:${name}` were neither edges nor
        # non-literal reports; every quoted-but-not-literal first argument is reported now.
        for body in (
            "await workflow('vsdd-factory:' + name, {})\n",
            "await workflow(`vsdd-factory:${name}`, {})\n",
        ):
            path = self.write_workflow("wave", body + "return {}\n")
            problems = workflow_problems(path, self.roster)
            self.assertTrue(any("not a literal workflow name" in p for p in problems), problems)

    def test_a_non_literal_call_target_is_reported(self) -> None:
        path = self.write_workflow("wave", "await workflow(target, {})\nreturn {}\n")
        self.assertTrue(any("not a literal workflow name" in p for p in workflow_problems(path, self.roster)))

    def test_story_may_not_call_runstories(self) -> None:
        # runStories() is a call of story; story is called by others and holds
        # no call of its own (pipeline.md 2.3), the prelude's own runStories
        # call aside.
        path = self.write_workflow("story", "await runStories(units)\nreturn {}\n")
        problems = workflow_problems(path, self.roster)
        self.assertTrue(any("does not list for 'story'" in p for p in problems), problems)

    def test_release_may_not_call_runstories(self) -> None:
        # The roster gives release calls = [].
        path = self.write_workflow("release", "await runStories(units)\nreturn {}\n")
        problems = workflow_problems(path, self.roster)
        self.assertTrue(any("does not list for 'release'" in p for p in problems), problems)

    def test_wave_may_call_runstories(self) -> None:
        path = self.write_workflow("wave", "await runStories(units)\nreturn {}\n")
        self.assertEqual(workflow_problems(path, self.roster), [])

    def test_call_edge_not_in_the_roster_is_rejected(self) -> None:
        path = self.write_workflow("wave", "await workflow('vsdd-factory:release', {})\nreturn {}\n")
        self.assertTrue(any("does not list for 'wave'" in p for p in workflow_problems(path, self.roster)))

    def test_called_workflow_may_not_call_itself(self) -> None:
        path = self.write_workflow("story", "await workflow('vsdd-factory:story', {})\nreturn {}\n")
        self.assertTrue(any("contains no workflow() call" in p for p in workflow_problems(path, self.roster)))

    def test_call_of_an_unregistered_workflow_is_rejected(self) -> None:
        path = self.write_workflow("wave", "await workflow('vsdd-factory:gossip', {})\nreturn {}\n")
        self.assertTrue(any("'gossip', which is not in the roster" in p for p in workflow_problems(path, self.roster)))

    def test_workflow_file_absent_from_the_roster_is_rejected(self) -> None:
        path = self.write_workflow("gossip", "return {}\n")
        self.assertTrue(any("not in the roster's [workflows]" in p for p in workflow_problems(path, self.roster)))

    def test_a_direct_agent_call_is_rejected(self) -> None:
        path = self.write_workflow("wave", "await agent('implementer', {})\nreturn {}\n")
        problems = workflow_problems(path, self.roster)
        self.assertTrue(any("direct agent(" in p for p in problems), problems)

    def test_setting_a_model_or_effort_is_rejected(self) -> None:
        path = self.write_workflow("wave", "const o = { model: 'opus' }\nreturn {}\n")
        problems = workflow_problems(path, self.roster)
        self.assertTrue(any("tier settings are the roster's" in p for p in problems), problems)

    def test_assigning_the_generated_tier_tables_is_rejected(self) -> None:
        for assignment in ("RUN.tier = 'planner'", "TIER.planner = {}", "ROLE = {}", "OVERRIDE = {}"):
            path = self.write_workflow("wave", assignment + "\nreturn {}\n")
            problems = workflow_problems(path, self.roster)
            self.assertTrue(any("tier settings are the roster's" in p for p in problems), assignment)

    def test_a_comment_and_a_prompt_string_are_not_tier_settings(self) -> None:
        # B6-R7: the scans read the code only, so a comment naming the agent and a prompt
        # string's 'effort:' trip nothing. (These two fail with the stripping removed.)
        path = self.write_workflow(
            "wave",
            "// hand the agent (x) its brief\nconst line = 'effort: high and model: opus'\nreturn {}\n",
        )
        self.assertEqual(workflow_problems(path, self.roster), [])

    def test_interpolations_are_scanned_as_code(self) -> None:
        # B6-R7: a ${...} interpolation of a template literal is code, so a direct agent(
        # and a forbidden workflow() call written there are reported like any other.
        path = self.write_workflow(
            "wave",
            "const line = `${await agent('x', {})}`\nreturn {}\n",
        )
        self.assertTrue(any("direct agent(" in p for p in workflow_problems(path, self.roster)))
        path = self.write_workflow(
            "wave",
            "const line = `${await workflow('vsdd-factory:release', {})}`\nreturn {}\n",
        )
        self.assertTrue(any("does not list for 'wave'" in p for p in workflow_problems(path, self.roster)))

    def test_a_regex_literal_hides_nothing_after_it(self) -> None:
        # B6-R7: a quote inside a regex literal must not throw the string scanner off; the
        # agent( call after s.replace(/'/g, '') is still reported.
        path = self.write_workflow(
            "wave",
            "const s = 'a b'.replace(/'/g, '')\nawait agent('x', {})\nreturn {}\n",
        )
        problems = workflow_problems(path, self.roster)
        self.assertTrue(any("direct agent(" in p for p in problems), problems)

    def test_the_skeleton_tier_adoptions_are_allowed(self) -> None:
        # B6-R7: the two adoption forms the skeletons and toys use pass; each adopts a tier
        # its caller or the survey passed, a dotted chain ending in || null.
        for assignment in (
            "RUN.tier = a.tier || null",
            "RUN.tier = s.config.tier_override || null",
            "RUN.tier = survey.config.tier_override || null",
        ):
            path = self.write_workflow("wave", assignment + "\nreturn {}\n")
            problems = [p for p in workflow_problems(path, self.roster) if "tier settings are the roster's" in p]
            self.assertEqual(problems, [], assignment)

    def test_pinning_the_run_to_a_tier_is_rejected(self) -> None:
        # B6-R7: the adoption exception is exactly the skeleton forms; pinning the run, from
        # a table name or without the `|| null`, is a tier setting and is rejected.
        for assignment in (
            "RUN.tier = TIER.planner",
            "RUN.tier = TIER.reviewer || null",
            "RUN.tier = a.tier || TIER.planner",
            "RUN.tier = a.tier",
            "RUN.tier = 'planner'",
        ):
            path = self.write_workflow("wave", assignment + "\nreturn {}\n")
            problems = workflow_problems(path, self.roster)
            self.assertTrue(any("tier settings are the roster's" in p for p in problems), assignment)

    def test_a_subscripted_or_merged_tier_write_is_rejected(self) -> None:
        # C1-1(a): the assignment scan sees the spelling RUN.tier only, so the subscript form
        # and a merge into RUN are violations in themselves.
        for assignment in ("RUN['tier'] = 'reviewer'", "Object.assign(RUN, { tier: TIER.reviewer })"):
            path = self.write_workflow("wave", assignment + "\nreturn {}\n")
            problems = workflow_problems(path, self.roster)
            self.assertTrue(any("tier settings are the roster's" in p for p in problems), assignment)

    def test_an_alias_of_the_tier_table_is_rejected(self) -> None:
        # C1-1(b): the adoption form admits a dotted chain, so renaming the table past the
        # assignment scan — const x = TIER, then RUN.tier = x.reviewer || null — worked; any
        # naming of TIER, ROLE or OVERRIDE in the hand-written region is a tier setting now.
        path = self.write_workflow("wave", "const x = TIER\nRUN.tier = x.reviewer || null\nreturn {}\n")
        problems = workflow_problems(path, self.roster)
        self.assertTrue(any("tier settings are the roster's" in p for p in problems), problems)

    def test_a_hand_written_tier_key_is_rejected(self) -> None:
        # C1-1(c): a call passes a story its tier, it never chooses one; the two adoption
        # forms carry no tier: key, so a tier: key in the hand-written region is rejected,
        # with a table reference as without one.
        for value in ("TIER.reviewer", "asked"):
            path = self.write_workflow(
                "wave",
                f"await workflow('vsdd-factory:story', {{ story: id, tier: {value} }})\nreturn {{}}\n",
            )
            problems = workflow_problems(path, self.roster)
            self.assertTrue(any("tier settings are the roster's" in p for p in problems), value)

    def test_a_model_chosen_between_literals_is_rejected(self) -> None:
        # C1-1(d): the narrowed key scan stopped at a value that begins with a name; a value
        # holding a quoted literal anywhere before the next , or } is a tier setting now,
        # and a carried value still passes.
        path = self.write_workflow("wave", "const out = { model: big ? 'opus' : 'sonnet' }\nreturn {}\n")
        problems = workflow_problems(path, self.roster)
        self.assertTrue(any("tier settings are the roster's" in p for p in problems), problems)
        path = self.write_workflow("wave", "const out = { model: big ? u.model : v.model }\nreturn {}\n")
        problems = [p for p in workflow_problems(path, self.roster) if "tier settings are the roster's" in p]
        self.assertEqual(problems, [])

    def test_an_unterminated_quote_hides_nothing_after_it(self) -> None:
        # C1-1: a / the stripper misreads — a division read as a regex, or a quote carried
        # into the next line — opened a string that blanked the rest of the file and hid a
        # later agent( call; a ' or " string ends at an unescaped newline now.
        for body in (
            "if (x) /'/.test(y)\nawait agent('implementer', {})\n",
            "a++ / 2; const s = 'x/y'\nawait agent('implementer', {})\n",
        ):
            path = self.write_workflow("wave", body + "return {}\n")
            problems = workflow_problems(path, self.roster)
            self.assertTrue(any("direct agent(" in p for p in problems), body)

    def test_a_carried_model_data_key_is_not_a_tier_setting(self) -> None:
        # B6-R7, decided: a data key whose value is carried ({ model: u.model }) passes through
        # what the run was given and is no tier setting; one set from a literal is.
        path = self.write_workflow("wave", "const out = { model: u.model, effort: u.effort }\nreturn {}\n")
        problems = [p for p in workflow_problems(path, self.roster) if "tier settings are the roster's" in p]
        self.assertEqual(problems, [])
        path = self.write_workflow("wave", "const out = { model: 'opus' }\nreturn {}\n")
        self.assertTrue(any("tier settings are the roster's" in p for p in workflow_problems(path, self.roster)))

    def test_step_and_relay_and_gated_are_not_direct_calls(self) -> None:
        body = (
            "await step('implementer', 'work', 'S-1.01', 'Do it.')\n"
            "await relay('x', {})\n"
            "await gated('implementer', 'w', 'S-1.01', 'Do it.', 'red', {})\n"
            "return {}\n"
        )
        path = self.write_workflow("wave", body)
        self.assertEqual(workflow_problems(path, self.roster), [])


class TestSpecSkeletons(unittest.TestCase):
    """The spec's own skeletons pass the workflow lint (B6-R7): they adopt RUN.tier from
    their inputs and set no tier of their own, and their comments and prompt strings trip
    no scan."""

    def _skeleton_problems(self, stem: str, relative: str) -> list[str]:
        text = (NEXT.parent / "rewrite" / "spec" / relative).read_text()
        blocks = re.findall(r"```js\n(.*?)```", text, re.S)
        assert blocks, relative
        body = max(blocks, key=len)  # the body block is the long one; prose fences are short
        tmp = tempfile.TemporaryDirectory(prefix="vsdd-skeleton-")
        self.addCleanup(tmp.cleanup)
        path = Path(tmp.name) / f"{stem}.js"
        path.write_text(
            f"export const meta = {{\n  name: '{stem}',\n  phases: [],\n}}\n"
            f"{build_mod.PRELUDE_MARKERS[0]}\n{build_mod.PRELUDE_MARKERS[1]}\n" + body,
            encoding="utf-8",
        )
        return workflow_problems(path, load_roster())

    def test_the_story_skeleton_passes_the_workflow_lint(self) -> None:
        self.assertEqual(self._skeleton_problems("story", "topics/story-script.md"), [])

    def test_the_wave_skeleton_passes_the_workflow_lint(self) -> None:
        self.assertEqual(self._skeleton_problems("wave", "topics/wave-script.md"), [])

    def test_the_toys_set_no_tier_of_their_own(self) -> None:
        # The toys adopt the tier their caller or survey passed (the two skeleton forms);
        # the lint must not report their RUN.tier lines, whatever their comments and
        # prompt strings say about agents, models and efforts.
        toys = NEXT / "tests" / "workflows" / "toys"
        roster = load_roster()
        for path in sorted(toys.glob("*.js")):
            problems = [p for p in workflow_problems(path, roster) if "tier settings are the roster's" in p]
            self.assertEqual(problems, [], path.name)


class TestReferenceResolution(unittest.TestCase):
    """The reference half of L-XREF over the files this lint reads."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="vsdd-reflint-")
        self.addCleanup(self._tmp.cleanup)
        self.plugin = Path(self._tmp.name) / "vsdd-factory"
        (self.plugin / "workflows").mkdir(parents=True)
        self.roster = load_roster()

    def test_a_reference_that_resolves_passes(self) -> None:
        path = self.plugin / "workflows" / "wave.js"
        path.write_text("await workflow('vsdd-factory:story', {})\n")
        self.assertEqual(reference_problems([path], self.roster), [])

    def test_a_reference_that_resolves_to_nothing_is_flagged(self) -> None:
        path = self.plugin / "workflows" / "wave.js"
        path.write_text("await workflow('vsdd-factory:gossip', {})\n")
        self.assertTrue(any("gossip" in p for p in reference_problems([path], self.roster)))


class TestManifestLint(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="vsdd-rosterlint-")
        self.addCleanup(self._tmp.cleanup)
        self.plugin = Path(self._tmp.name) / "vsdd-factory"
        (self.plugin / ".claude-plugin").mkdir(parents=True)
        (self.plugin / ".claude-plugin" / "icon.png").write_bytes(b"png")
        (self.plugin / "README.md").write_text("readme")
        self.roster = load_roster()

    def write_manifest(self, engines: str, icon: str = "./.claude-plugin/icon.png") -> Path:
        path = self.plugin / ".claude-plugin" / "plugin.json"
        path.write_text(f'{{"name": "vsdd-factory", "engines": {{"claude-code": "{engines}"}}, "icon": "{icon}"}}')
        return path

    def test_engines_floor_may_drift_from_the_roster(self) -> None:
        self.write_manifest(">=2.0.0")
        self.assertTrue(any(">=" in p and "2.1.331" in p for p in manifest_problems(self.plugin, self.roster)))

    def test_engines_floor_matches_the_roster(self) -> None:
        self.write_manifest(f">={self.roster['platform']['min_claude_code']}")
        self.assertEqual(manifest_problems(self.plugin, self.roster), [])

    def test_missing_icon_file_is_rejected(self) -> None:
        self.write_manifest(f">={self.roster['platform']['min_claude_code']}", icon="./.claude-plugin/missing.png")
        self.assertTrue(any("does not exist" in p for p in manifest_problems(self.plugin, self.roster)))


class TestRosterIntegrity(unittest.TestCase):
    """roster_problems: the roster's tables point at each other."""

    def setUp(self) -> None:
        self.roster = load_roster()

    def test_the_live_roster_is_coherent(self) -> None:
        self.assertEqual(roster_problems(self.roster), [])

    def test_an_agent_tier_outside_the_tiers_is_rejected(self) -> None:
        roster = self.roster
        roster["agents"]["clerk"] = "chief"
        self.assertTrue(any("chief" in p for p in roster_problems(roster)))

    def test_a_skill_mode_outside_the_enumeration_is_rejected(self) -> None:
        roster = self.roster
        roster["skills"]["status"] = "everyone"
        self.assertTrue(any("everyone" in p for p in roster_problems(roster)))

    def test_an_override_to_an_unknown_tier_is_rejected(self) -> None:
        roster = self.roster
        roster["overrides"]["ingest/map"] = "planner-emeritus"
        self.assertTrue(any("planner-emeritus" in p for p in roster_problems(roster)))

    def test_an_override_for_an_unknown_workflow_is_rejected(self) -> None:
        roster = self.roster
        roster["overrides"]["gossip/map"] = "builder"
        self.assertTrue(any("gossip" in p for p in roster_problems(roster)))

    def test_a_call_of_an_unknown_workflow_is_rejected(self) -> None:
        roster = self.roster
        roster["workflows"]["wave"]["calls"] = ["gossip"]
        self.assertTrue(any("gossip" in p for p in roster_problems(roster)))

    def test_a_called_workflow_with_its_own_calls_is_rejected(self) -> None:
        roster = self.roster
        roster["workflows"]["story"]["calls"] = ["discover"]
        self.assertTrue(any("story' is called by another workflow" in p for p in roster_problems(roster)))

    def test_a_missing_budget_key_is_reported(self) -> None:
        roster = self.roster
        del roster["budgets"]["agent_file_lines"]
        self.assertTrue(any("agent_file_lines" in p for p in roster_problems(roster)))


class TestRosterMembership(unittest.TestCase):
    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="vsdd-rosterlint-")
        self.addCleanup(self._tmp.cleanup)
        self.plugin = Path(self._tmp.name) / "vsdd-factory"
        for name in ("agents", "skills", "workflows"):
            (self.plugin / name).mkdir(parents=True)
        self.roster = load_roster()

    def test_agent_absent_from_the_roster_is_rejected(self) -> None:
        (self.plugin / "agents" / "ghost-writer.md").write_text("---\nname: ghost-writer\n---\n")
        problems = membership_problems(self.plugin, self.roster)
        self.assertTrue(any("ghost-writer" in p and "[agents]" in p for p in problems))

    def test_skill_directory_absent_from_the_roster_is_rejected(self) -> None:
        (self.plugin / "skills" / "conjure").mkdir()
        problems = membership_problems(self.plugin, self.roster)
        self.assertTrue(any("conjure" in p and "[skills]" in p for p in problems))

    def test_the_roster_lists_the_whole_design(self) -> None:
        self.assertEqual(len(self.roster["agents"]), 20)
        self.assertEqual(len(self.roster["skills"]), 13)
        self.assertEqual(len(self.roster["workflows"]), 12)
        self.assertEqual(len(self.roster["tiers"]), 4)
        self.assertEqual(roster_problems(self.roster), [])
        for kind, calls in (
            ("feature", ["story"]),
            ("verify", ["story"]),
            ("maintain", ["story"]),
            ("multi", ["story"]),
        ):  # AU-P2-01: the shipped roster says all four call story (pipeline.md 2.3)
            self.assertEqual(self.roster["workflows"][kind]["calls"], calls)

    @staticmethod
    def _populate(plugin: Path, roster: dict) -> None:
        """Every roster name present on disk, as E1 part 2 will leave the plugin."""
        for name in roster["agents"]:
            (plugin / "agents" / f"{name}.md").write_text("---\nname: ghost-writer\n---\n", encoding="utf-8")
        for name in roster["skills"]:
            (plugin / "skills" / name).mkdir()
        for name in roster["workflows"]:
            (plugin / "workflows" / f"{name}.js").write_text("export const meta = {}\n", encoding="utf-8")

    def test_a_populated_plugin_leaves_nothing_absent(self) -> None:
        # B6-R12: the roster-side absence check, exercised on fixtures. A plugin
        # with every roster file present passes.
        self._populate(self.plugin, self.roster)
        self.assertEqual(absence_problems(self.plugin, self.roster), [])

    def test_a_missing_agent_file_is_reported(self) -> None:
        self._populate(self.plugin, self.roster)
        (self.plugin / "agents" / "product-owner.md").unlink()
        problems = absence_problems(self.plugin, self.roster)
        self.assertTrue(any("product-owner" in p for p in problems), problems)
        self.assertEqual(len(problems), 1, problems)

    def test_a_missing_skill_directory_and_workflow_are_reported(self) -> None:
        self._populate(self.plugin, self.roster)
        first_skill = next(iter(self.roster["skills"]))
        first_workflow = next(iter(self.roster["workflows"]))
        (self.plugin / "skills" / first_skill).rmdir()
        (self.plugin / "workflows" / f"{first_workflow}.js").unlink()
        problems = absence_problems(self.plugin, self.roster)
        self.assertTrue(any(f"'{first_skill}'" in p for p in problems), problems)
        self.assertTrue(any(f"'{first_workflow}'" in p for p in problems), problems)

    @unittest.skip("switches on with E1 part 2 (packaging/build-plan.md): agents/, skills/ and workflows/ fill then")
    def test_nothing_in_the_roster_is_absent_from_disk(self) -> None:
        # The other half of L-ROSTER over the real plugin. The logic is pinned by
        # the fixture tests above, so switching this on at E1 part 2 is deleting
        # this decorator, nothing else (B6-R12).
        self.assertEqual(absence_problems(PLUGIN, self.roster), [])


class TestKindGrantLint(unittest.TestCase):
    """L-KIND: the story kinds' stage agents hold the write grants their outputs need."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="vsdd-kindlint-")
        self.addCleanup(self._tmp.cleanup)
        self.policy = Path(self._tmp.name) / "policy"
        self.policy.mkdir(parents=True)

    def write_policy(self, artifacts: str, walls: str) -> list[str]:
        (self.policy / "artifacts.toml").write_text(artifacts, encoding="utf-8")
        (self.policy / "walls.toml").write_text(walls, encoding="utf-8")
        return kind_grant_problems(self.policy)

    ARTIFACT = """\
[[story-kind]]
id = "feature"
tests_agent = "test-writer"
tests_method = "failing-tests"
implement_agent = "implementer"
implement_method = "tdd"
red_gate = true

[[story-kind]]
id = "chore"
tests_agent = ""
tests_method = ""
implement_agent = "devops"
implement_method = "repo-setup"
red_gate = false
"""

    WALLS = """\
["vsdd-factory:test-writer"]
write = ["tests:frozen", "manifests", "source:before-red"]

["vsdd-factory:implementer"]
write = ["source", "manifests"]

["vsdd-factory:devops"]
write = ["ci", "repo-config", "manifests", "source:create@repo-setup", "tests:create@repo-setup"]
"""

    def test_consistent_kinds_and_walls_pass(self) -> None:
        self.assertEqual(self.write_policy(self.ARTIFACT, self.WALLS), [])

    def test_a_missing_class_grant_is_rejected(self) -> None:
        walls = self.WALLS.replace(', "manifests"]', "]", 1)  # implementer loses manifests
        problems = self.write_policy(self.ARTIFACT, walls)
        self.assertTrue(any("feature" in p and "'manifests'" in p for p in problems), problems)

    def test_repo_setup_needs_the_qualified_grants(self) -> None:
        walls = self.WALLS.replace(', "source:create@repo-setup", "tests:create@repo-setup"', "", 1)
        problems = self.write_policy(self.ARTIFACT, walls)
        self.assertTrue(any("chore" in p and "'source' in a repo-setup story" in p for p in problems), problems)

    def test_a_conditional_qualifier_does_not_meet_an_unconditional_need(self) -> None:
        # 'source:before-red' is write-before-red only, so it does not satisfy
        # the implementer's unconditional need for source.
        walls = self.WALLS.replace('write = ["source", "manifests"]', 'write = ["source:before-red", "manifests"]')
        problems = self.write_policy(self.ARTIFACT, walls)
        self.assertTrue(any("feature" in p and "'source'" in p for p in problems), problems)

    def test_zero_story_kinds_is_a_problem(self) -> None:
        problems = self.write_policy("# no kinds\n", self.WALLS)
        self.assertTrue(any("no [[story-kind]] rows" in p for p in problems), problems)

    def test_an_empty_implement_agent_is_a_problem(self) -> None:
        artifacts = self.ARTIFACT.replace('implement_agent = "devops"\nimplement_method = "repo-setup"',
                                          'implement_agent = ""\nimplement_method = "repo-setup"')
        problems = self.write_policy(artifacts, self.WALLS)
        self.assertTrue(any("names no implement agent" in p for p in problems), problems)


class TestWallsRowsMatchTheRoster(unittest.TestCase):
    """L-AGENT's closing clause: every agent has a walls.toml row and every row has an agent."""

    def setUp(self) -> None:
        self._tmp = tempfile.TemporaryDirectory(prefix="vsdd-wallslint-")
        self.addCleanup(self._tmp.cleanup)
        self.policy = Path(self._tmp.name) / "policy"
        self.policy.mkdir(parents=True)
        self.roster = load_roster()

    def test_matching_rows_pass(self) -> None:
        walls = "".join(
            f'["vsdd-factory:{name}"]\nwrite = []\n\n' for name in self.roster["agents"]
        )
        (self.policy / "walls.toml").write_text(walls, encoding="utf-8")
        self.assertEqual(walls_row_problems(self.policy, self.roster), [])

    def test_a_missing_row_is_reported(self) -> None:
        (self.policy / "walls.toml").write_text("")
        problems = walls_row_problems(self.policy, self.roster)
        self.assertTrue(any("no row for the roster agent" in p and "clerk" in p for p in problems), problems)

    def test_an_extra_row_is_reported(self) -> None:
        walls = "".join(
            f'["vsdd-factory:{name}"]\nwrite = []\n\n' for name in self.roster["agents"]
        ) + '["vsdd-factory:adversry"]\nwrite = []\n'
        (self.policy / "walls.toml").write_text(walls, encoding="utf-8")
        problems = walls_row_problems(self.policy, self.roster)
        self.assertTrue(any("adversry" in p for p in problems), problems)


class TestLiveTree(unittest.TestCase):
    """The lint, over the real plugin tree: what the layer asserts on every run."""

    def test_plugin_tree_is_clean(self) -> None:
        roster = load_roster()
        problems = check_all(PLUGIN, roster, _stamping_sources())
        self.assertEqual(problems, [], problems)

    def test_roster_agrees_with_the_manifest_floor(self) -> None:
        manifest = json.loads((PLUGIN / ".claude-plugin" / "plugin.json").read_text())
        roster = load_roster()
        self.assertEqual(manifest["engines"]["claude-code"], f">={roster['platform']['min_claude_code']}")


if __name__ == "__main__":
    unittest.main()
