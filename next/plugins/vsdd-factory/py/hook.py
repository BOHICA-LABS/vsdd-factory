# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Entry point for hook events; runs the rule engine's main().

This file is the `uv run --script` target that py/launch.sh starts. The hook
runner lives in the vsdd package under py/vsdd/hooks/ (H1).

Contract (rewrite/design/hooks.md 2.2):
  exit 0   allow (stdout holds at most one JSON object, or plain text on
           PreCompact)
  exit 10  a rule blocked (the reason is already on stderr; the launcher maps
           it to exit 2 for Claude Code)
  exit 13  the interpreter is older than the 3.11 floor (defence in depth;
           the launcher already passes --python '>=3.11' to uv)
  any other code is the runner's own crash, which the launcher maps
"""

import sys
from pathlib import Path

# PYTHONSAFEPATH=1 (set by py/launch.sh) stops `uv run --script` from putting
# this directory on sys.path, which would hide the sibling vsdd package. The
# launcher cannot do this for us (hooks.md 2.2 and 2.4).
sys.path.insert(0, str(Path(__file__).resolve().parent))

FLOOR = (3, 11)


def main(argv: list[str]) -> int:
    if sys.version_info[:2] < FLOOR:
        print(
            "vsdd-factory: the hook runner needs Python 3.11 or newer; "
            f"this is {'.'.join(str(p) for p in sys.version_info[:2])}.",
            file=sys.stderr,
        )
        return 13

    from vsdd.hooks import main as runner

    return runner.main(argv)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
