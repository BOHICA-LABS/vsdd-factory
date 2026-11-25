"""A fixture rule that blocks: proves the exit-10 path and the four lines."""

from vsdd.hooks.model import Block


def check(ctx, cfg):
    return Block(
        "fixture_block",
        "the fixture rule blocks this call",
        "do the fixture thing instead",
    )
