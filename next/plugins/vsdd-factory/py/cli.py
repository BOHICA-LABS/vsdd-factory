# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Entry point for the `vsdd` command.

Started by py/launch.sh as `uv run --script cli.py <args>` (and reachable
through bin/vsdd). The operator subcommands are added package by package; this
unit delivers the argument parser frame, the global `-C <dir>` option slot and
`--version`. The version is read from the plugin manifest, which is its single
source.

Exit codes (blueprint 4.9): 0 success or "yes"; 1 a negative answer; 2 a
usage or environment error.
"""

import argparse
import json
import sys
from pathlib import Path

# PYTHONSAFEPATH=1 (set by py/launch.sh) stops `uv run --script` from putting
# this directory on sys.path, which would hide the sibling vsdd package. The
# launcher cannot do this for us (hooks.md 2.2 and 2.4).
sys.path.insert(0, str(Path(__file__).resolve().parent))

PROGRAM = "vsdd"


def plugin_version() -> str:
    manifest = Path(__file__).resolve().parent.parent / ".claude-plugin" / "plugin.json"
    try:
        with open(manifest, encoding="utf-8") as fh:
            return str(json.load(fh)["version"])
    except (OSError, KeyError, json.JSONDecodeError):
        # An unusable manifest is an environment error, not a crash: exit 2
        # with the name of the faulty installation.
        print(f"{PROGRAM}: cannot read the plugin manifest at {manifest}; the plugin installation is damaged. Reinstall the plugin.", file=sys.stderr)
        raise SystemExit(2)


def build_parser(version: str) -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog=PROGRAM,
        description="vsdd-factory operator tool (subpackage commands arrive with their packages).",
    )
    parser.add_argument("--version", action="version", version=f"{PROGRAM} {version}")
    parser.add_argument(
        "-C",
        metavar="DIR",
        help="project directory (global option; honoured by the subcommands)",
    )
    return parser


def main(argv: list[str]) -> int:
    version = plugin_version()
    parser = build_parser(version)
    parser.parse_args(argv)  # --version and -h exit from here
    # No subcommand was given. The subcommand table is empty in this unit, so
    # this is a usage error; outside-initialised-project handling arrives with
    # the subcommands themselves.
    parser.print_usage(sys.stderr)
    print(f"{PROGRAM}: no subcommand given; try '{PROGRAM} --help'", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
