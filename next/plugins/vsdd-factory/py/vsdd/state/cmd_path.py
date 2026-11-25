"""``vsdd path <id>``: print the path of one identified artifact
(command:path). Derived view; writes nothing."""

from __future__ import annotations

from ..cli.dispatch import Command, Reply


def _configure(parser) -> None:
    parser.add_argument("ident", help="the artifact's identifier, e.g. S-9.01")


def _run(inv, args) -> Reply:
    from . import artifacts

    ident = args.ident
    hit = artifacts.path_for(ident)
    if hit is None:
        return Reply(
            ok=False,
            code=1,
            text=f"vsdd path: no artifact kind matches {ident}",
            data={"id": ident, "kind": "", "path": "", "exists": False},
        )
    kind, rel = hit
    exists = (inv.root / rel).exists() if inv.root is not None else False
    if not exists:
        return Reply(
            ok=False,
            code=1,
            text=f"vsdd path: no {kind} artifact {ident} exists at {rel}",
            data={"id": ident, "kind": kind, "path": rel, "exists": False},
        )
    return Reply(
        ok=True,
        text=rel,  # without --json it prints the path alone
        data={"id": ident, "kind": kind, "path": rel, "exists": True},
    )


COMMANDS = (
    Command(
        words=("path",),
        summary="print the path of an identified artifact",
        run=_run,
        configure=_configure,
        needs="initialised",
    ),
)
