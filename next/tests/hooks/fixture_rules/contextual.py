"""A fixture rule that adds context, with and without a system message."""

from vsdd.hooks.model import Context


def check(ctx, cfg):
    return Context("the fixture context", "vsdd-factory: fixture system line")
