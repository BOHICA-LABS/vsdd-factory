"""The runner's main: read one call, build the context, run the rules selected
for its event and tool under the deadline, and turn their outcomes into the
exit code, stdout and stderr Claude Code sees (plan 3.1; HD3, HD5, HD6, HD11).

Two ways in: :func:`main` reads the payload from stdin (``hook.py``'s call),
and :func:`run` takes the payload text and environment, which is what the
tests drive. Both return the exit code: 0, or 10 when a rule blocked.
"""

from __future__ import annotations

import json
import os
import signal
import sys
import time
from pathlib import Path

from .. import events as _events
from . import caller, emit, registry
from .model import (
    Advise,
    Block,
    Context,
    Ctx,
    Rewrite,
    RuleConfig,
    RuleEntry,
    EVENT_CONFIG_INVALID,
    EVENT_POLICY_INVALID,
    EVENT_RULE_BLOCK,
    EVENT_RULE_ERROR,
    EVENT_RULE_TIMEOUT,
    EVENT_RULE_WOULD_BLOCK,
    EVENT_RUNNER_CALL,
    EVENT_RUNNER_SLOW,
)

__all__ = ["main", "run"]

#: The fixed fallback briefing of brief mode (HD3): a crash, the deadline or
#: unreadable input never silences the session start.
FALLBACK_TEXT = "vsdd-factory: the session briefing could not run; the gates still run."
FALLBACK_SYSTEM = "vsdd-factory: the session briefing failed"

_QUARANTINE_ENV = "VSDD_FACTORY_HOOKS_QUARANTINE"
_DEBUG_ENV = "VSDD_FACTORY_HOOKS_DEBUG"
_CAPTURE_ENV = "VSDD_FACTORY_HOOKS_CAPTURE"

_SLOW_MS = 300


class _Deadline(BaseException):
    """Raised by the alarm inside whatever rule is running (HD4: a
    ``BaseException``, so a rule's ``except Exception`` cannot swallow it)."""


def main(argv: list[str]) -> int:
    """``argv[0]`` is the mode; the payload arrives on stdin."""
    mode = argv[0] if argv else "gate"
    raw = sys.stdin.read()
    return run(mode, raw, dict(os.environ))


def run(
    mode: str,
    payload_text: str,
    env: dict,
    *,
    entries: list[RuleEntry] | tuple[RuleEntry, ...] | None = None,
    stdout=None,
    stderr=None,
) -> int:
    """One hook call, in process. Returns 0, or 10 when something blocked."""
    with _streams(stdout, stderr):
        return _run_guarded(mode, payload_text, env, entries)


def _run_guarded(mode, payload_text, env, entries) -> int:
    root = env.get("VSDD_FACTORY_ROOT") or None
    if not _root_usable(root, env):
        return 0  # the runner is inert outside a factory project

    payload = _parse_payload(payload_text)
    if payload is None:
        return _bad_input(mode, root, env)  # HD11: not a JSON object, or no event name

    ctx = Ctx(payload, env, mode, root)
    started = ctx.started

    spec = _event_spec(ctx.event)
    outcomes: list[tuple[str, object]] = []

    if ctx.event == "SessionStart":
        outcomes.extend(_notices(ctx))

    # Outside a factory project the hooks are inert ([[topic:hooks-inert]]),
    # decided once here, before any rule is selected or loaded (ORCH-6): the
    # runner's own selection chooses nothing, so nothing blocks, nothing is
    # printed and nothing is logged. Explicit ``entries`` — the seam the
    # tests drive a rule's mechanics through — are honoured as given.
    governing = ctx.project

    timed_out_rule = None
    crashed = False
    with _alarm(spec.deadline if spec else None) as alarm:
        chosen = _entries(entries)
        selected = (
            registry.select(ctx.event, ctx.tool, chosen)
            if (governing is not None or chosen is not None)
            else []
        )
        for entry in selected:
            ctx.git_budget_reset()
            alarm.current = entry.id
            try:
                if not _applies(entry, ctx):
                    continue
                ctx.project  # the governing project, resolved once here
                results, rule_crashed = _run_rule(entry, ctx, outcomes)
                crashed = crashed or rule_crashed
            except _Deadline:
                timed_out_rule = entry
                break
            except Exception as exc:
                crashed = True
                # The deadline may strike while the crash is being logged:
                # then this call is a timeout, never an escaped alarm (HD4).
                try:
                    _rule_crashed(entry, ctx, exc, outcomes)
                except _Deadline:
                    timed_out_rule = entry
                    break
                continue
            if results is not None:
                for single in results:
                    outcomes.append((entry.id, single))
        alarm.current = None

    timed_out = timed_out_rule is not None
    if timed_out:  # HD3: the deadline is a block in gate, a log everywhere
        seconds = spec.deadline if spec else 0
        if mode == "gate":
            outcomes.append(
                (
                    timed_out_rule.id,
                    Block(
                        "runner_timeout",
                        f"the {timed_out_rule.id} rule was still running when its "
                        f"{seconds:g} s deadline passed; the rules after it did not run.",
                        _QUARANTINE_FIX.format(rule=timed_out_rule.id),
                    ),
                )
            )
        ctx.log(EVENT_RULE_TIMEOUT, "error", rule=timed_out_rule.id, error="deadline", fail="closed")

    if mode == "brief" and (timed_out or crashed):
        # The fixed fallback: never a silent session start (HD3, HD6). Fired
        # by the crash and timeout flags themselves, not by outcome codes: in
        # brief a crash or deadline appends no Block of its own.
        outcomes.append(("runner", Context(FALLBACK_TEXT, FALLBACK_SYSTEM)))

    if not any(isinstance(outcome, Block) for _, outcome in outcomes):
        marker_outcome = _marker_outcome(ctx, outcomes)
        if marker_outcome is not None:
            outcomes.append(marker_outcome)

    _liveness(ctx)

    code = emit.finish(ctx, outcomes, mode)
    _debug(ctx, started, code, payload_text, env)
    return code


def _entries(entries):
    return tuple(entries) if entries is not None else None


# --- one rule ---------------------------------------------------------------


def _rule_cfg(entry: RuleEntry, ctx: Ctx) -> RuleConfig:
    """The configuration of one rule, with the locked set enforced (plan 3.2:
    a locked rule always gets ``block``, whatever its table says). The locked
    set is declared twice — here through the row, and in
    ``vsdd.config._LOCKED_HOOK_RULES`` for the loader's own validation — and
    tests/lint/test_hooks_wiring.py checks the two agree."""
    cfg = ctx.project.rule_config(entry.id)
    if entry.locked and cfg.strength != "block":
        cfg = RuleConfig("block", cfg.options)
    return cfg


def _run_rule(entry: RuleEntry, ctx: Ctx, outcomes) -> tuple[list, bool]:
    """Load and run one rule; return its results and whether it crashed. A
    crash becomes its declared failure (HD1, HD3). At advise strength a block
    relaxes to advice, logged as ``rule.would_block`` — the gate advised, it
    did not switch off."""
    try:
        check = entry.load()
        cfg = _rule_cfg(entry, ctx)
        results = _as_list(check(ctx, cfg))
    except Exception as exc:
        _rule_crashed(entry, ctx, exc, outcomes)
        return [], True
    kept = []
    for item in results:
        if isinstance(item, Rewrite) and not entry.rewrites:
            # HD10, at every strength: a rule may return a rewrite only when
            # its row sets rewrites; otherwise it is that rule's crash.
            _rule_crashed(
                entry,
                ctx,
                RuntimeError("returned Rewrite but its row does not set rewrites"),
                outcomes,
            )
            continue
        kept.append(item)
    if cfg.strength != "advise":
        return kept, False
    relaxed = []
    for item in kept:
        if isinstance(item, Block):
            ctx.log(
                EVENT_RULE_WOULD_BLOCK,
                "warn",
                id=emit.new_block_id(),
                rule=entry.id,
                code=item.code,
                reason=item.reason,
                fix=item.fix,
                **emit.context_target_fields(ctx),
            )
            relaxed.append(Advise(item.code, item.reason))
        else:
            relaxed.append(item)
    return relaxed, False


def _rule_crashed(entry: RuleEntry, ctx: Ctx, exc: Exception, outcomes) -> None:
    """One crash: logged in every mode; a fail-closed rule blocks the gate.
    A policy fault carries its own code: ``policy_invalid`` for the rules that
    read the table ([[topic:hook-failure]])."""
    fail = entry.fail
    ctx.log(EVENT_RULE_ERROR, "error", rule=entry.id, error=f"{type(exc).__name__}: {exc}", fail=fail)
    if ctx.mode == "gate" and fail == "closed":
        from .policy import PolicyInvalid

        code = "policy_invalid" if isinstance(exc, PolicyInvalid) else "rule_error"
        outcomes.append(
            (
                entry.id,
                Block(
                    code,
                    f"the {entry.id} rule crashed: {type(exc).__name__}: {exc}",
                    _QUARANTINE_FIX.format(rule=entry.id),
                ),
            )
        )


_QUARANTINE_FIX = (
    "Code with VSDD_FACTORY_HOOKS_QUARANTINE={rule} set and report the fault. "
    "Restart Claude Code afterwards."
)


def _as_list(outcome) -> list:
    if outcome is None:
        return []
    return outcome if isinstance(outcome, list) else [outcome]


def _applies(entry: RuleEntry, ctx: Ctx) -> bool:
    """HD5, in order: the quarantine, then strength ``off``, then the spike
    exemption. Runs inside the rule's ``try``: a fault here is its crash. A
    locked rule is never off: :func:`_rule_cfg` forces ``block``."""
    if _quarantined(entry, ctx):
        return False
    cfg = _rule_cfg(entry, ctx)
    if cfg.strength == "off":
        return False
    if entry.spike_exempt and ctx.in_spike:
        return False
    return True


def _quarantined(entry: RuleEntry, ctx: Ctx) -> bool:
    """HD5: the switch names the rule and the two newest daily log files show
    two or more ``rule.error`` or ``rule.timeout`` events for it."""
    wanted = ctx.env.get(_QUARANTINE_ENV) or ""
    if wanted != entry.id:
        return False
    return _recent_rule_faults(ctx) >= 2


def _events_dir(ctx: Ctx) -> Path:
    """The log directory of the call: the environment override wins, as the
    event reader itself honours it for tests."""
    return _events_dir_of(ctx.env, ctx.root)


def _recent_rule_faults(ctx: Ctx) -> int:
    try:
        directory = _events_dir(ctx)
        days = sorted(
            (path for path in directory.glob("*.jsonl") if path.is_file()),
            reverse=True,
        )[:2]
        count = 0
        for path in days:
            for line in path.read_text(encoding="utf-8").splitlines():
                try:
                    event = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if isinstance(event, dict) and event.get("type") in (
                    EVENT_RULE_ERROR,
                    EVENT_RULE_TIMEOUT,
                ) and event.get("rule") == ctx.env.get(_QUARANTINE_ENV):
                    count += 1
        return count
    except OSError:
        return 0


# --- session notices (HD6) ---------------------------------------------------


def _notices(ctx: Ctx) -> list[tuple[str, object]]:
    """Until H9's briefing exists the core says four things on SessionStart,
    each a ``Context`` with ``system`` set; ``config.invalid`` and
    ``policy.invalid`` are logged here only."""
    notices: list[tuple[str, object]] = []
    project = ctx.project
    if project is None:
        return notices

    error = project.config_error
    problems = project.config.hook_problems
    try:
        rules_table = project.config.get("hooks", "rules") or {}
    except (KeyError, TypeError):
        rules_table = {}

    # Unknown rule ids are a configuration fault (rule-tuning.md): a typo must
    # not read as a working relaxation. Reported beside the loader's problems;
    # the loader itself leaves the check to the runner (config.py).
    registered = {entry.id for entry in registry.RULES}
    unknown = sorted(set(rules_table) - registered)

    if error is not None or problems or unknown:
        detail = f"{error}" if error is not None else ""
        for rule_id, rule_problems in sorted(problems.items()):
            detail += f"; [{rule_id}] { '; '.join(rule_problems)}"
        detail = detail.strip("; ")
        if unknown:
            detail = ", ".join(filter(None, (detail, "unknown rules: " + ", ".join(unknown))))
        notices.append(
            (
                "runner",
                Context(
                    "vsdd-factory: the project configuration is invalid "
                    f"({detail}) ; built-in defaults, the strictest "
                    "settings, are in force until it is fixed.",
                    "vsdd-factory: the project configuration is invalid",
                ),
            )
        )
        ctx.log(EVENT_CONFIG_INVALID, "warn", detail=detail)

    from .policy import PolicyInvalid

    try:
        project.policy
    except PolicyInvalid as exc:
        notices.append(
            (
                "runner",
                Context(
                    f"vsdd-factory: a policy table is invalid ({exc}); the rules "
                    "that need it fail closed until it is restored.",
                    "vsdd-factory: a policy table is invalid",
                ),
            )
        )
        ctx.log(EVENT_POLICY_INVALID, "warn", detail=str(exc))

    quarantined = ctx.env.get(_QUARANTINE_ENV) or ""
    if quarantined and _recent_rule_faults(ctx) >= 2:
        notices.append(
            (
                "runner",
                Context(
                    f"vsdd-factory: a quarantine is in force: the {quarantined} rule "
                    "is skipped this session (it has crashed repeatedly). The gates "
                    "still run; announce this to the person.",
                    "vsdd-factory: a rule is quarantined",
                ),
            )
        )

    relaxed = []
    for rule_id, options in sorted(rules_table.items()):
        # Only registered rules are announced as relaxations; an unknown id is
        # the configuration fault above, never a working relaxation.
        if rule_id not in registered:
            continue
        if isinstance(options, dict) and options.get("strength") in ("advise", "off"):
            relaxed.append(f"{rule_id} ({options.get('strength')})")
    if relaxed:
        notices.append(
            (
                "runner",
                Context(
                    "vsdd-factory: rules below full strength this session: "
                    + ", ".join(relaxed)
                    + ". Relaxations are announced at every session start.",
                    "vsdd-factory: rules are below full strength",
                ),
            )
        )
    return notices


# --- the caller marker --------------------------------------------------------


def _marker_outcome(ctx: Ctx, outcomes) -> tuple[str, object] | None:
    """For a vsdd call no rule blocked: the marker and the rewrite (plan 3.5).
    A Bash command that types ``VSDD_CALLER=`` itself blocks (HD9), and so does
    a call whose marker cannot be written."""
    if ctx.event != "PreToolUse" or ctx.tool != "Bash":
        return None
    command = ctx.command
    if command is None:
        return None
    if caller.forged(str(ctx.tool_input.get("command", ""))):
        return (
            "runner",
            Block(
                "caller_forged",
                "this command sets VSDD_CALLER itself; the caller marker is the "
                "runner's to write, and a forged marker would impersonate a session.",
                "Run the vsdd command as typed; the runner adds the marker itself.",
            ),
        )
    if not caller.is_vsdd_call(command):
        return None
    try:
        return ("__runner__", caller.marker_rewrite(ctx, command))
    except caller.MarkerUnwritable as exc:
        return (
            "runner",
            Block(
                "caller_marker_unwritable",
                f"the caller marker could not be written ({exc}); a vsdd call that "
                "passes without it would report the hooks as dead.",
                "Make .factory/local writable for the session's user.",
            ),
        )


# --- lock liveness (hook-runner-b 3.2) ----------------------------------------


def _liveness(ctx: Ctx) -> None:
    """The lock liveness duties, wired with no rule (HD2, D1): the refresh on
    every ``PreToolUse`` call, after the rules and whatever they decided, and
    the release on ``SessionEnd``, whatever its reason. Neither can block a
    call or change its output: every fault is caught inside :mod:`liveness`
    and logged as ``runner.liveness_error`` (D5)."""
    from . import liveness

    if ctx.event == "PreToolUse":
        liveness.refresh(ctx)
    elif ctx.event == "SessionEnd":
        liveness.release(ctx)


# --- plumbing ------------------------------------------------------------------


def _root_usable(root: str | None, env: dict) -> bool:
    """The runner runs only inside a factory project whose root is not the
    home directory or ``/`` (plan 3.1: inert there)."""
    if not root:
        return False
    real = os.path.realpath(root)
    if real == "/" :
        return False
    home = env.get("HOME")
    if home and real == os.path.realpath(home):
        return False
    return True


def _parse_payload(payload_text: str) -> dict | None:
    try:
        payload = json.loads(payload_text)
    except (json.JSONDecodeError, TypeError):
        return None
    if not isinstance(payload, dict) or not payload.get("hook_event_name"):
        return None
    return payload


def _event_spec(event: str):
    for spec in registry.EVENTS:
        if spec.name == event:
            return spec
    return None


def _events_dir_of(env: dict, root: str | None) -> Path | None:
    """The events directory a call logs to: the ``VSDD_EVENTS_DIR`` override
    wins, as the event reader itself honours for tests."""
    override = env.get("VSDD_EVENTS_DIR")
    if override:
        return Path(override)
    return _events.events_dir_for(root) if root else None


def _bad_input(mode: str, root: str | None, env: dict) -> int:
    """HD11: not a JSON object, or no ``hook_event_name``. Under the session
    id ``unknown``, gate blocks loudly and logs the block; notify and brief
    block nothing, and log the fault as a ``rule.error`` of the runner (the
    failure table: logged in notify). The ``VSDD_EVENTS_DIR`` override is
    honoured, as the event reader itself does."""
    block_id = emit.new_block_id()
    message = Block(
        "runner_bad_input",
        "the hook payload could not be read: the runner needs one JSON object "
        "with hook_event_name on stdin.",
        "Re-run the command; if it repeats, reinstall the plugin.",
    )
    if mode == "gate":
        try:
            directory = _events_dir_of(env, root)
            if directory is not None:
                log = _events.EventLog(directory, "unknown")
                log.log(
                    EVENT_RULE_BLOCK,
                    "block",
                    id=block_id,
                    rule="runner",
                    code=message.code,
                    reason=message.reason,
                    fix=message.fix,
                )
        except Exception:
            pass  # the event log cannot take the decision down with it
        for line in emit.block_lines("", "runner", message.code, message.reason, message.fix, block_id):
            print(line, file=sys.stderr)
        return 10
    # Notify and brief: nothing blocks, but the fault is on the record (HD11).
    try:
        directory = _events_dir_of(env, root)
        if directory is not None:
            log = _events.EventLog(directory, "unknown")
            log.log(
                EVENT_RULE_ERROR,
                "error",
                rule="runner",
                code=message.code,
                error=message.reason,
                fail="open",
            )
    except Exception:
        pass  # the event log cannot take the call down with it
    if mode == "brief":
        print(
            json.dumps(
                {
                    "continue": True,
                    "hookSpecificOutput": {
                        "hookEventName": "SessionStart",
                        "additionalContext": FALLBACK_TEXT,
                    },
                    "systemMessage": FALLBACK_SYSTEM,
                },
                ensure_ascii=False,
            ),
            file=sys.stdout,
        )
    return 0


def _debug(ctx: Ctx, started: float, code: int, payload_text: str, env: dict) -> None:
    """The environment switches (plan 3.2 of the design's table; the
    environment table): ``VSDD_FACTORY_HOOKS_DEBUG`` logs ``runner.call``, and
    ``VSDD_FACTORY_HOOKS_CAPTURE`` stores the payload for the test suite. A
    call over 300 ms is logged as ``runner.slow`` whatever the DEBUG switch
    says (tables/event-types.toml)."""
    if env.get(_CAPTURE_ENV):
        try:
            _capture(payload_text, env)
        except OSError:
            pass  # a capture failure is diagnostic, never behavioural
    ms = int((time.monotonic() - started) * 1000)
    if ms > _SLOW_MS:
        ctx.log(EVENT_RUNNER_SLOW, "warn", code=f"exit_{code}", mode=ctx.mode, ms=ms)
    if env.get(_DEBUG_ENV) != "1":
        return
    fields = {"code": f"exit_{code}", "mode": ctx.mode, "ms": ms}
    ctx.log(EVENT_RUNNER_CALL, "info", **fields)


def _capture(payload_text: str, env: dict) -> None:
    import secrets

    directory = Path(env[_CAPTURE_ENV])
    root = env.get("VSDD_FACTORY_ROOT", "")
    home = env.get("HOME", "")
    here = Path(__file__).resolve().parents[3]
    text = payload_text.replace(root, "{root}").replace(home, "{home}").replace(str(here), "{plugin}")
    try:
        payload = json.loads(text)
    except json.JSONDecodeError:
        return
    event = payload.get("hook_event_name", "Unknown") if isinstance(payload, dict) else "Unknown"
    tool = payload.get("tool_name", "") if isinstance(payload, dict) else ""
    target = directory / str(event)
    target.mkdir(parents=True, exist_ok=True)
    # Unique per capture: two calls of one tool must not overwrite each other.
    stamp = f"{int(time.time() * 1000):013d}-{secrets.token_hex(4)}"
    name = f"{tool or 'call'}-{stamp}.json"
    (target / name).write_text(text, encoding="utf-8")
    meta = directory / "_meta.json"
    if not meta.exists():
        meta.write_text(
            json.dumps(
                {
                    "captured_by": "vsdd-factory hook runner",
                    "version": _version(),
                    "captured_at": _timestamp(),
                    "substitutions": ["{root}", "{home}", "{plugin}"],
                },
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )


def _version() -> str:
    """The plugin's version, from its manifest; ``unknown`` when absent."""
    manifest = Path(__file__).resolve().parents[3] / ".claude-plugin" / "plugin.json"
    try:
        return str(json.loads(manifest.read_text(encoding="utf-8")).get("version", "unknown"))
    except (OSError, ValueError):
        return "unknown"


def _timestamp() -> str:
    from datetime import datetime, timezone

    return (
        datetime.now(timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z")
    )


class _Alarm:
    """One alarm per call (HD4): the deadline values are the event row's."""

    def __init__(self, seconds: float | None):
        self.seconds = seconds
        self.current: str | None = None
        self._previous = None

    def __enter__(self) -> "_Alarm":
        if self.seconds:
            self._previous = signal.getsignal(signal.SIGALRM)
            signal.signal(signal.SIGALRM, _raise_deadline)
            signal.setitimer(signal.ITIMER_REAL, self.seconds)
        return self

    def __exit__(self, *exc_info) -> bool:
        if self.seconds:
            signal.setitimer(signal.ITIMER_REAL, 0.0)
            if self._previous is not None:
                signal.signal(signal.SIGALRM, self._previous)
        return False


def _raise_deadline(signum, frame) -> None:
    raise _Deadline()


def _alarm(seconds: float | None) -> _Alarm:
    return _Alarm(seconds)


class _Streams:
    def __init__(self, stdout, stderr):
        self.stdout = stdout
        self.stderr = stderr

    def __enter__(self):
        self.saved = (sys.stdout, sys.stderr)
        if self.stdout is not None:
            sys.stdout = self.stdout
        if self.stderr is not None:
            sys.stderr = self.stderr
        return self

    def __exit__(self, *exc_info) -> bool:
        sys.stdout, sys.stderr = self.saved
        return False


def _streams(stdout, stderr) -> _Streams:
    return _Streams(stdout, stderr)
