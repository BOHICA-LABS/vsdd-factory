"""A fixture rule that permits the call."""

from vsdd.hooks.model import Permit


def check(ctx, cfg):
    return Permit("the fixture rule permits this call")
