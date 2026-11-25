"""``vsdd validate``: check artifacts of the state tree against the registry
(command:validate). One syntax: ``vsdd validate [--changed] [PATH...]``."""

from __future__ import annotations

from pathlib import Path

from ..cli.dispatch import Command, Reply, Refuse


def _configure(parser) -> None:
    parser.add_argument("--changed", action="store_true", help="check the changed files only")
    parser.add_argument("paths", nargs="*", help="state files to check, relative to the root")


def _run(inv, args) -> Reply:
    from . import artifacts

    root = inv.root
    if args.paths and args.changed:
        raise Refuse("give either --changed or paths, not both")
    if args.paths:
        rels = []
        for given in args.paths:
            path = Path(given)
            if not path.is_absolute():
                path = inv.start / path
            try:
                rel = path.resolve().relative_to(root.resolve()).as_posix()
            except ValueError:
                raise Refuse(f"{given} is not inside the project") from None
            if not (root / rel).is_file():
                raise Refuse(f"{given} is not a file in the project")
            rels.append(rel)
    elif args.changed:
        rels = artifacts.changed_paths(root)
    else:
        rels = None
    # Only files of the artifact territories are checked; the count says so.
    if rels is None:
        rels = artifacts.tree_rels(root)
    checked = sum(1 for rel in rels if artifacts.kind_of(rel) is not None)
    problems = artifacts.check_paths(root, rels, config=inv.config, references=True)
    errors = [p for p in problems if p.severity == "error"]
    warnings = [p for p in problems if p.severity == "warning"]
    return Reply(
        ok=not errors,
        code=0 if not errors else 1,
        data={
            "checked": checked,
            "problems": [
                {
                    "path": p.rel,
                    "code": p.code,
                    "message": p.message,
                    "severity": p.severity,
                }
                for p in problems
            ],
            "warnings": [
                {"path": p.rel, "code": p.code, "message": p.message} for p in warnings
            ],
        },
    )


COMMANDS = (
    Command(
        words=("validate",),
        summary="check artifacts against the registry",
        run=_run,
        configure=_configure,
        needs="initialised",
    ),
)
