"""``vsdd config --json``: print the merged project configuration as data
(command:config). Read-only; the briefing and the survey read it."""

from __future__ import annotations

from ..cli.dispatch import Command, Reply


def _run(inv, _args) -> Reply:
    config = inv.config  # a ConfigError reaches the dispatcher: exit 2
    return Reply(
        data={"config": config.data, "hook_problems": dict(config.hook_problems)},
    )


COMMANDS = (
    Command(
        words=("config",),
        summary="print the project configuration as data",
        run=_run,
        needs="initialised",
    ),
)
