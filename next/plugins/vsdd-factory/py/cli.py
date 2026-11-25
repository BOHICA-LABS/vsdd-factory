# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Entry point for the `vsdd` command.

Started by py/launch.sh as `uv run --script cli.py <args>` (and reachable
through bin/vsdd). The commands themselves live in modules under
`vsdd.state`, `vsdd.gates`, `vsdd.delivery` and `vsdd.cli`, dispatched by
`vsdd.cli.dispatch` (state-core.md 3.1): this file holds the guarded import
and the guarded call that turn a broken installation or a killed run into
exit 2 — an import error or a crash must not leave as exit 1, the launcher's
negative answer.

Exit codes (blueprint 4.9): 0 success or "yes"; 1 a negative answer; 2 a
usage or environment error.
"""

import sys
from pathlib import Path

# PYTHONSAFEPATH=1 (set by py/launch.sh) stops `uv run --script` from putting
# this directory on sys.path, which would hide the sibling vsdd package. The
# launcher cannot do this for us (hooks.md 2.2 and 2.4).
sys.path.insert(0, str(Path(__file__).resolve().parent))

PROGRAM = "vsdd"


def main(argv: list[str] | None = None, **kwargs) -> int:
    """One whole run of vsdd; `vsdd <words>` argument tokens in, exit code out."""
    if argv is None:
        argv = sys.argv[1:]
    try:
        from vsdd.cli import dispatch
    except BaseException as exc:
        if isinstance(exc, SystemExit):
            raise
        print(f"{PROGRAM}: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 2
    try:
        return dispatch.main(list(argv), **kwargs)
    except SystemExit:
        raise
    except BaseException as exc:
        # Anything at all that escapes the dispatcher is an environment error,
        # not the negative answer: exit 2 with the name of the fault.
        print(f"{PROGRAM}: {type(exc).__name__}: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
